use crate::{Endpoint, Middleware, Request, Result};

/// Middleware for adding any data to a request.
pub struct AddData<T> {
    value: T,
}

impl<T: Clone + Send + Sync + 'static> AddData<T> {
    /// Create new `AddData` middleware with any value.
    pub fn new(value: T) -> Self {
        AddData { value }
    }
}

impl<E, T, S> Middleware<E, S> for AddData<T>
where
    E: Endpoint<S>,
    T: Clone + Send + Sync + 'static,
    S: Send + Sync,
{
    type Output = AddDataEndpoint<E, T>;

    fn transform(&self, ep: E) -> Self::Output {
        AddDataEndpoint {
            inner: ep,
            value: self.value.clone(),
        }
    }
}

/// Endpoint for the AddData middleware.
pub struct AddDataEndpoint<E, T> {
    inner: E,
    value: T,
}

impl<E, T, S> Endpoint<S> for AddDataEndpoint<E, T>
where
    E: Endpoint<S>,
    T: Clone + Send + Sync + 'static,
    S: Send + Sync,
{
    type Output = E::Output;

    async fn call(&self, mut req: Request, state: &S) -> Result<Self::Output> {
        req.extensions_mut().insert(self.value.clone());
        self.inner.call(req, state).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EndpointExt, handler, test::TestClient};

    #[tokio::test]
    async fn test_add_data() {
        #[handler(internal)]
        async fn index(req: &Request) {
            assert_eq!(req.extensions().get::<i32>(), Some(&100));
        }

        // Explicitly specify state type () for the endpoint chain
        let ep = EndpointExt::<()>::with(index, AddData::new(100i32));
        let cli = TestClient::new(ep);
        cli.get("/").send().await.assert_status_is_ok();
    }
}
