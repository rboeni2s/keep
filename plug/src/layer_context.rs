use crate::{
    dispatch::{LayerDispatch, NoDispatch},
    registry::Registry,
};
use keep::{Guard, Heap, Keep};
use std::any::TypeId;


pub type StaticContext<E = NoDispatch, Err = (), Res = ()> =
    std::sync::LazyLock<LayerContext<E, Err, Res>>;


#[macro_export]
macro_rules! dep_vec
{
    ($($dep:ident),*) => {vec![$(::std::any::TypeId::of::<$dep>()),*]};
}


#[macro_export]
macro_rules! static_context {
    ($layer:ident) => {::std::sync::LazyLock::new(|| $crate::layer_context::LayerContext::new::<$layer>($crate::dep_vec![]))};
    ($layer:ident, [$($dep:ident),*] ) => {::std::sync::LazyLock::new(|| $crate::layer_context::LayerContext::new::<$layer>($crate::dep_vec![$($dep),*]))};
}


pub trait LayerConstruct<E = NoDispatch, Err = (), Res = ()>
where
    Self: Sized,
{
    fn construct(registry: &Registry<E, Err, Res>) -> Self;
}


pub trait LayerConstructor<E, Err, Res>
{
    fn constructor()
    -> impl Fn(&Registry<E, Err, Res>) -> Box<dyn LayerDispatch<E, Error = Err, Response = Res>>
    + 'static;
}


impl<E, Err, Res, T> LayerConstructor<E, Err, Res> for T
where
    E: 'static,
    Err: 'static,
    Res: 'static,
    T: LayerConstruct<E, Err, Res> + LayerDispatch<E, Error = Err, Response = Res> + 'static,
{
    fn constructor()
    -> impl Fn(&Registry<E, Err, Res>) -> Box<dyn LayerDispatch<E, Error = Err, Response = Res>>
    {
        |registry| Box::new(Self::construct(registry))
    }
}


#[allow(clippy::type_complexity)]
pub struct LayerContext<E, Err, Res>
{
    type_id: TypeId,
    deps: Vec<TypeId>,
    constructor: Guard<
        Box<
            dyn Fn(
                &Registry<E, Err, Res>,
            ) -> Box<dyn LayerDispatch<E, Error = Err, Response = Res>>,
        >,
    >,
}


unsafe impl<E, Err, Res> Send for LayerContext<E, Err, Res> {}
unsafe impl<E, Err, Res> Sync for LayerContext<E, Err, Res> {}


impl<E, Err, Res> LayerContext<E, Err, Res>
{
    pub fn new<C>(deps: Vec<TypeId>) -> Self
    where
        C: LayerConstructor<E, Err, Res> + 'static,
    {
        #[allow(clippy::type_complexity)]
        let constructor: Box<
            dyn Fn(
                    &Registry<E, Err, Res>,
                )
                    -> Box<dyn LayerDispatch<E, Error = Err, Response = Res> + 'static>
                + 'static,
        > = Box::new(C::constructor());

        let constructor = unsafe { Heap::from_ptr(Box::into_raw(Box::new(constructor))) };
        let constructor: Keep<Box<_>> = Keep::new(constructor);

        Self {
            type_id: TypeId::of::<C>(),
            deps,
            constructor: constructor.read(),
        }
    }

    pub(crate) fn insert_into_reg(&self, reg: &Registry<E, Err, Res>)
    {
        unsafe {
            reg.insert_by((self.constructor)(reg), self.type_id);
        };
    }
    pub(crate) fn deps(&self) -> Vec<TypeId>
    {
        self.deps.clone()
    }

    pub(crate) fn id(&self) -> TypeId
    {
        self.type_id
    }
}
