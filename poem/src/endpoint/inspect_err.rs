use std::marker::PhantomData;

use crate::{Endpoint, Request, Result};

/// Endpoint for the
/// [`inspect_err`](super::EndpointExt::inspect_err) method.
pub struct InspectError<E, F, ErrType, S = ()> {
    inner: E,
    f: F,
    _mark: PhantomData<(ErrType, S)>,
}

impl<E, F, ErrType, S> InspectError<E, F, ErrType, S> {
    #[inline]
    pub(crate) fn new(inner: E, f: F) -> InspectError<E, F, ErrType, S> {
        Self {
            inner,
            f,
            _mark: PhantomData,
        }
    }
}

impl<E, F, ErrType, S> Endpoint<S> for InspectError<E, F, ErrType, S>
where
    E: Endpoint<S>,
    F: Fn(&ErrType) + Send + Sync,
    ErrType: std::error::Error + Send + Sync + 'static,
    S: Send + Sync,
{
    type Output = E::Output;

    async fn call(&self, req: Request, state: &S) -> Result<Self::Output> {
        match self.inner.call(req, state).await {
            Ok(resp) => Ok(resp),
            Err(err) if err.is::<ErrType>() => {
                (self.f)(err.downcast_ref::<ErrType>().unwrap());
                Err(err)
            }
            Err(err) => Err(err),
        }
    }
}
