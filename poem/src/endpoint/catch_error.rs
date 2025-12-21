use std::{future::Future, marker::PhantomData};

use crate::{Endpoint, IntoResponse, Request, Response, Result};

/// Endpoint for the [`catch_error`](super::EndpointExt::catch_error) method.
pub struct CatchError<E, F, R, ErrType, S = ()> {
    inner: E,
    f: F,
    _mark: PhantomData<(R, ErrType, S)>,
}

impl<E, F, R, ErrType, S> CatchError<E, F, R, ErrType, S> {
    #[inline]
    pub(crate) fn new(inner: E, f: F) -> CatchError<E, F, R, ErrType, S> {
        Self {
            inner,
            f,
            _mark: PhantomData,
        }
    }
}

impl<E, F, Fut, R, ErrType, S> Endpoint<S> for CatchError<E, F, R, ErrType, S>
where
    E: Endpoint<S>,
    F: Fn(ErrType) -> Fut + Send + Sync,
    Fut: Future<Output = R> + Send,
    R: IntoResponse + Send + Sync,
    ErrType: std::error::Error + Send + Sync + 'static,
    S: Send + Sync,
{
    type Output = Response;

    async fn call(&self, req: Request, state: &S) -> Result<Self::Output> {
        match self.inner.call(req, state).await {
            Ok(resp) => Ok(resp.into_response()),
            Err(err) if err.is::<ErrType>() => Ok((self.f)(err.downcast::<ErrType>().unwrap())
                .await
                .into_response()),
            Err(err) => Err(err),
        }
    }
}
