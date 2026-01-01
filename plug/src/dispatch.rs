pub enum NoDispatch {}


pub trait LayerDispatch<E>
{
    type Error;
    type Response;
    fn layer_dispatch(&self, event: &E) -> Result<Self::Response, Self::Error>;
}

pub trait SimpleDispatch<E>
{
    fn simple_dispatch(&self, _event: &E) {}
}


impl<T> SimpleDispatch<NoDispatch> for T
{
    fn simple_dispatch(&self, _event: &NoDispatch) {}
}


impl<E, T: SimpleDispatch<E>> LayerDispatch<E> for T
{
    type Error = ();
    type Response = ();

    fn layer_dispatch(&self, event: &E) -> Result<Self::Response, Self::Error>
    {
        self.simple_dispatch(event);
        Ok(())
    }
}
