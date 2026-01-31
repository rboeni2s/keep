use crate::{
    dispatch::NoDispatch,
    layer_context::LayerContext,
    prelude::{ConstructLayer, ConstructLayerEx},
    registry::Registry,
};
use std::{any::TypeId, collections::HashSet, hash::Hash};


struct Ctx<E, Err, Res>
{
    layer: LayerContext<E, Err, Res>,
    deps: Vec<TypeId>,
}


impl<E, Err, Res> Eq for Ctx<E, Err, Res> {}
impl<E, Err, Res> PartialEq for Ctx<E, Err, Res>
{
    fn eq(&self, other: &Self) -> bool
    {
        self.layer.id() == other.layer.id()
    }
}


impl<E, Err, Res> Hash for Ctx<E, Err, Res>
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H)
    {
        self.layer.id().hash(state);
    }
}


impl<E, Err, Res> Ctx<E, Err, Res>
{
    fn new(layer: LayerContext<E, Err, Res>) -> Self
    {
        Self {
            deps: layer.deps().iter().map(|ctx| ctx.id()).collect(),
            layer,
        }
    }
}


pub struct Resolver<E = NoDispatch, Err = (), Res = ()>
{
    layers: Vec<Ctx<E, Err, Res>>,
}


impl<E, Err, Res> Resolver<E, Err, Res>
{
    pub fn new() -> Self
    {
        Resolver { layers: Vec::new() }
    }

    pub fn add_ctx<L: ConstructLayer<E, Err, Res>>(mut self) -> Self
    {
        self.collect_deps(L::ctx());
        self
    }

    fn collect_deps(&mut self, layer: LayerContext<E, Err, Res>)
    {
        for dep in layer.deps()
        {
            self.collect_deps(dep.clone());
        }

        self.layers.push(Ctx::new(layer));
    }

    pub fn build_reg(mut self) -> Option<Registry<E, Err, Res>>
    {
        // This is not a very efficient algorithm, but it does not need to be because
        // it should only run at registry instantiation which is not performance critical.

        let reg = Registry::new();

        let mut clean_layers = HashSet::<&mut _>::from_iter(self.layers.iter_mut())
            .drain()
            .collect::<Vec<_>>();


        loop
        {
            let mut resolved = None;

            for (i, Ctx { deps, .. }) in clean_layers.iter().enumerate()
            {
                if deps.is_empty()
                {
                    resolved = Some(i);
                    break;
                }
            }

            match resolved
            {
                Some(index) =>
                {
                    // Remove the resolved layer from the other layers
                    let resolved_layer = clean_layers.remove(index).layer.clone();
                    let dep = resolved_layer.id();

                    // Remove the resolved layer from the other layers dependencies
                    for Ctx { deps, .. } in &mut clean_layers
                    {
                        deps.retain(|e| *e != dep);
                    }

                    // Add the layer to the reg
                    resolved_layer.insert_into_reg(&reg);
                }

                None =>
                {
                    if clean_layers.is_empty()
                    {
                        return Some(reg);
                    }

                    return None;
                }
            }
        }
    }
}


impl<E, Err, Res> Default for Resolver<E, Err, Res>
{
    fn default() -> Self
    {
        Self::new()
    }
}
