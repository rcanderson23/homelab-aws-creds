use http::{Request, Response};
use http_body::Body;
use pin_project_lite::pin_project;
use std::{
    future::Future,
    pin::Pin,
    task::{ready, Context, Poll},
};
use tower_layer::Layer;
use tower_service::Service;

#[derive(Clone, Debug)]
pub struct MetricsInterceptor<S> {
    inner: S,
}

#[derive(Clone, Debug)]
pub struct MetricsLayer;

#[allow(dead_code)]
impl MetricsLayer {
    pub fn new() -> Self {
        Self
    }
}
impl<S> Layer<S> for MetricsLayer {
    type Service = MetricsInterceptor<S>;

    fn layer(&self, inner: S) -> Self::Service {
        MetricsInterceptor { inner }
    }
}

impl<S, R, ResBody> Service<Request<R>> for MetricsInterceptor<S>
where
    S: Service<Request<R>, Response = Response<ResBody>>,
{
    type Response = Response<ResponseBody<ResBody>>;
    type Error = S::Error;
    type Future = ResponseFuture<S::Future>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<R>) -> Self::Future {
        let uri = req.uri();
        let path = uri.path().to_owned();
        metrics::counter!("requests_count", "path" => path).increment(1);
        ResponseFuture {
            inner: self.inner.call(req),
        }
    }
}

pin_project! {
    /// Response future for [`InFlightRequests`].
    pub struct ResponseFuture<F> {
        #[pin]
        inner: F,
    }
}

impl<F, B, E> Future for ResponseFuture<F>
where
    F: Future<Output = Result<Response<B>, E>>,
{
    type Output = Result<Response<ResponseBody<B>>, E>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let response = ready!(this.inner.poll(cx))?;
        let response = response.map(move |body| ResponseBody { inner: body });

        Poll::Ready(Ok(response))
    }
}

pin_project! {
    /// Response body for [`InFlightRequests`].
    pub struct ResponseBody<B> {
        #[pin]
        inner: B,
    }
}

impl<B> Body for ResponseBody<B>
where
    B: Body,
{
    type Data = B::Data;
    type Error = B::Error;

    #[inline]
    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<http_body::Frame<Self::Data>, Self::Error>>> {
        self.project().inner.poll_frame(cx)
    }

    #[inline]
    fn is_end_stream(&self) -> bool {
        self.inner.is_end_stream()
    }

    #[inline]
    fn size_hint(&self) -> http_body::SizeHint {
        self.inner.size_hint()
    }
}
