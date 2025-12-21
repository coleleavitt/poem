use std::{future::Future, marker::PhantomData, sync::Arc};

use crate::{Endpoint, IntoResponse, Request, Result};

/// Endpoint for the [`around`](super::EndpointExt::around) method.
pub struct Around<E, F, S> {
    inner: Arc<E>,
    f: F,
    _mark: PhantomData<S>,
}

impl<E, F, S> Around<E, F, S> {
    #[inline]
    pub(crate) fn new(inner: E, f: F) -> Around<E, F, S> {
        Self {
            inner: Arc::new(inner),
            f,
            _mark: PhantomData,
        }
    }
}

impl<E, F, Fut, T, S> Endpoint<S> for Around<E, F, S>
where
    E: Endpoint<S>,
    F: Fn(Arc<E>, Request, S) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<T>> + Send,
    T: IntoResponse,
    S: Clone + Send + Sync + 'static,
{
    type Output = T;

    async fn call(&self, req: Request, state: &S) -> Result<Self::Output> {
        (self.f)(self.inner.clone(), req, state.clone()).await
    }
}
