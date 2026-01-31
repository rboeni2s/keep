use crate::{
    dispatch::{LayerDispatch, NoDispatch},
    registry::Registry,
};
use keep::Guard;
use std::any::TypeId;


type ConstructorFn<E, Err, Res> = Box<
    dyn Fn(
            &Registry<E, Err, Res>,
        ) -> Box<dyn LayerDispatch<E, Error = Err, Response = Res> + 'static>
        + 'static,
>;


pub trait ConstructLayer<E = NoDispatch, Err = (), Res = ()>
where
    Self: LayerDispatch<E, Error = Err, Response = Res> + Sized + 'static,
{
    fn construct(reg: &Registry<E, Err, Res>) -> Self;

    fn deps() -> Vec<LayerContext<E, Err, Res>>
    {
        vec![]
    }
}


/// # Safety
/// Do not alter the blanket implementations of `ctx()` and `type_context()`.
pub unsafe trait ConstructLayerEx<E, Err, Res>: ConstructLayer<E, Err, Res>
{
    fn ctx() -> LayerContext<E, Err, Res>
    {
        LayerContext::new::<Self>()
    }

    fn type_context() -> TypeId
    {
        TypeId::of::<Self>()
    }
}

unsafe impl<T, E, Err, Res> ConstructLayerEx<E, Err, Res> for T where T: ConstructLayer<E, Err, Res> {}


pub struct LayerContext<E = NoDispatch, Err = (), Res = ()>
{
    type_id: TypeId,
    deps: Vec<Self>,
    constructor: Guard<ConstructorFn<E, Err, Res>>,
}


impl<E, Err, Res> Clone for LayerContext<E, Err, Res>
{
    fn clone(&self) -> Self
    {
        Self {
            type_id: self.type_id,
            deps: self.deps.clone(),
            constructor: self.constructor.clone(),
        }
    }
}


impl<E, Err, Res> LayerContext<E, Err, Res>
{
    pub fn new<T: ConstructLayer<E, Err, Res>>() -> Self
    {
        let constructor: ConstructorFn<E, Err, Res> =
            Box::new(|reg: &Registry<E, Err, Res>| Box::new(T::construct(reg)));

        Self {
            type_id: T::type_context(),
            deps: T::deps(),
            constructor: Guard::new(constructor),
        }
    }

    pub(crate) fn insert_into_reg(&self, reg: &Registry<E, Err, Res>)
    {
        unsafe {
            reg.insert_by((self.constructor)(reg), self.type_id);
        };
    }

    pub(crate) fn deps(&self) -> &[Self]
    {
        &self.deps
    }

    pub(crate) fn id(&self) -> TypeId
    {
        self.type_id
    }
}
