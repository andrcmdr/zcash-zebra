use super::{
    error::Closed,
    message::{self, Message},
    BatchControl,
};
use futures::future::TryFutureExt;
use pin_project::pin_project;
use std::{
    marker::PhantomData,
    sync::{Arc, Mutex},
};
use tokio::{
    stream::StreamExt,
    sync::mpsc,
    time::{delay_for, Delay},
};
use tower::{Service, ServiceExt};
use tracing_futures::Instrument;

/// Task that handles processing the buffer. This type should not be used
/// directly, instead `Buffer` requires an `Executor` that can accept this task.
///
/// The struct is `pub` in the private module and the type is *not* re-exported
/// as part of the public API. This is the "sealed" pattern to include "private"
/// types in public traits that are not meant for consumers of the library to
/// implement (only call).
#[pin_project]
#[derive(Debug)]
pub struct Worker<S, Request, E2>
where
    S: Service<BatchControl<Request>>,
    S::Error: Into<E2>,
{
    rx: mpsc::Receiver<Message<Request, S::Future, S::Error>>,
    service: S,
    failed: Option<S::Error>,
    handle: Handle<S::Error, E2>,
    max_items: usize,
    max_latency: std::time::Duration,
    _e: PhantomData<E2>,
}

/// Get the error out
#[derive(Debug)]
pub(crate) struct Handle<E, E2> {
    inner: Arc<Mutex<Option<E>>>,
    _e: PhantomData<E2>,
}

impl<S, Request, E2> Worker<S, Request, E2>
where
    S: Service<BatchControl<Request>>,
    S::Error: Into<E2> + Clone,
{
    pub(crate) fn new(
        service: S,
        rx: mpsc::Receiver<Message<Request, S::Future, S::Error>>,
        max_items: usize,
        max_latency: std::time::Duration,
    ) -> (Handle<S::Error, E2>, Worker<S, Request, E2>) {
        let handle = Handle {
            inner: Arc::new(Mutex::new(None)),
            _e: PhantomData,
        };

        let worker = Worker {
            rx,
            service,
            handle: handle.clone(),
            failed: None,
            max_items,
            max_latency,
            _e: PhantomData,
        };

        (handle, worker)
    }

    async fn process_req(&mut self, req: Request, tx: message::Tx<S::Future, S::Error>) {
        if let Some(failed) = self.failed.clone() {
            tracing::trace!("notifying caller about worker failure");
            let _ = tx.send(Err(failed));
        } else {
            match self.service.ready_and().await {
                Ok(svc) => {
                    let rsp = svc.call(req.into());
                    let _ = tx.send(Ok(rsp));
                }
                Err(e) => {
                    self.failed(e);
                    let _ = tx.send(Err(self
                        .failed
                        .clone()
                        .expect("Worker::failed did not set self.failed?")));
                }
            }
        }
    }

    async fn flush_service(&mut self) {
        if let Err(e) = self
            .service
            .ready_and()
            .and_then(|svc| svc.call(BatchControl::Flush))
            .await
        {
            self.failed(e);
        }
    }

    pub async fn run(mut self) {
        use futures::future::Either::{Left, Right};
        // The timer is started when the first entry of a new batch is
        // submitted, so that the batch latency of all entries is at most
        // self.max_latency. However, we don't keep the timer running unless
        // there is a pending request to prevent wakeups on idle services.
        let mut timer: Option<Delay> = None;
        let mut pending_items = 0usize;
        loop {
            match timer {
                None => match self.rx.next().await {
                    // The first message in a new batch.
                    Some(msg) => {
                        let span = msg.span;
                        self.process_req(msg.request, msg.tx)
                            // Apply the provided span to request processing
                            .instrument(span)
                            .await;
                        timer = Some(delay_for(self.max_latency));
                        pending_items = 1;
                    }
                    // No more messages, ever.
                    None => return,
                },
                Some(delay) => {
                    // Wait on either a new message or the batch timer.
                    match futures::future::select(self.rx.next(), delay).await {
                        Left((Some(msg), delay)) => {
                            let span = msg.span;
                            self.process_req(msg.request, msg.tx)
                                // Apply the provided span to request processing.
                                .instrument(span)
                                .await;
                            pending_items += 1;
                            // Check whether we have too many pending items.
                            if pending_items >= self.max_items {
                                // XXX(hdevalence): what span should instrument this?
                                self.flush_service().await;
                                // Now we have an empty batch.
                                timer = None;
                                pending_items = 0;
                            } else {
                                // The timer is still running, set it back!
                                timer = Some(delay);
                            }
                        }
                        // No more messages, ever.
                        Left((None, _delay)) => {
                            return;
                        }
                        // The batch timer elapsed.
                        Right(((), _next)) => {
                            // XXX(hdevalence): what span should instrument this?
                            self.flush_service().await;
                            timer = None;
                            pending_items = 0;
                        }
                    }
                }
            }
        }
    }

    fn failed(&mut self, error: S::Error) {
        // The underlying service failed when we called `poll_ready` on it with
        // the given `error`. We need to communicate this to all the `Buffer`
        // handles. To do so, we require that `S::Error` implements `Clone`,
        // clone the error to send to all pending requests, and store it so that
        // subsequent requests will also fail with the same error.

        // Note that we need to handle the case where some handle is concurrently trying to send us
        // a request. We need to make sure that *either* the send of the request fails *or* it
        // receives an error on the `oneshot` it constructed. Specifically, we want to avoid the
        // case where we send errors to all outstanding requests, and *then* the caller sends its
        // request. We do this by *first* exposing the error, *then* closing the channel used to
        // send more requests (so the client will see the error when the send fails), and *then*
        // sending the error to all outstanding requests.

        let mut inner = self.handle.inner.lock().unwrap();

        if inner.is_some() {
            // Future::poll was called after we've already errored out!
            return;
        }

        *inner = Some(error.clone());
        drop(inner);

        self.rx.close();

        // By closing the mpsc::Receiver, we know that that the run() loop will
        // drain all pending requests. We just need to make sure that any
        // requests that we receive before we've exhausted the receiver receive
        // the error:
        self.failed = Some(error);
    }
}

impl<E, E2> Handle<E, E2>
where
    E: Clone + Into<E2>,
    crate::error::Closed: Into<E2>,
{
    pub(crate) fn get_error_on_closed(&self) -> E2 {
        self.inner
            .lock()
            .unwrap()
            .clone()
            .map(Into::into)
            .unwrap_or_else(|| Closed::new().into())
    }
}

impl<E, E2> Clone for Handle<E, E2> {
    fn clone(&self) -> Handle<E, E2> {
        Handle {
            inner: self.inner.clone(),
            _e: PhantomData,
        }
    }
}
