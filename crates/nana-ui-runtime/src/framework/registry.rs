//! AppContext registry operations.

use super::*;

impl AppContext {
    pub fn resolve_component_tag(&self, tag: &str) -> Option<&ComponentTypeId> {
        self.components.resolve_tag(tag)
    }

    /// Every component type this context can instantiate.
    #[cfg(test)]
    pub(crate) fn component_type_ids(&self) -> impl Iterator<Item = &ComponentTypeId> {
        self.components.type_ids()
    }

    /// Resolve an already-normalized tag (see [`normalize_tag`]).
    pub fn resolve_component_tag_normalized(
        &self,
        normalized_tag: &str,
    ) -> Option<&ComponentTypeId> {
        self.components.resolve_normalized(normalized_tag)
    }

    /// Whether the component registered under `type_id` reads the
    /// [`SemanticSpec`] fields a host derives by scanning a widget's children:
    /// `slots` (probing each child for `data-slot`) and `icon` (probing each
    /// child for an `Icon` kind).
    ///
    /// This lets a host skip both scans for a component that ignores them.
    /// Unknown types answer `true`.
    pub fn component_reads_child_derived_spec(&self, type_id: &ComponentTypeId) -> bool {
        self.components.reads_child_derived_spec(type_id)
    }

    pub fn bind_semantic(
        &self,
        id: StableNodeId,
        spec: &SemanticSpec<'_>,
        mutations: &mut MutationQueue,
    ) -> Result<ComponentBindKind, FrameworkError> {
        self.prepare_semantic_binding(id, spec, mutations)
            .map(|binding| binding.kind())
    }

    /// Stage a registry component once, preserving opted-in interaction state.
    pub fn prepare_semantic_binding(
        &self,
        id: StableNodeId,
        spec: &SemanticSpec<'_>,
        mutations: &mut MutationQueue,
    ) -> Result<crate::PreparedSemanticBinding, FrameworkError> {
        let mut request = ComponentBindRequest {
            id,
            world: &self.world,
            mutations,
            spec,
            previous: self.views.get(&id).map(|view| view.as_ref()),
            retained: None,
            finish: None,
        };
        let kind = self.components.bind(&mut request)?;
        request
            .mutations
            .set_component_type(id, Some(spec.type_id.clone()));
        Ok(crate::PreparedSemanticBinding {
            id,
            type_id: spec.type_id.clone(),
            kind,
            retained: request.retained,
            finish: request.finish,
        })
    }

    /// Install typed state after its UiWorld projection was committed. Does not
    /// project or parse a second time; optional assembly belongs to the type.
    pub fn finish_semantic_binding(
        &mut self,
        binding: crate::PreparedSemanticBinding,
    ) -> Result<(), FrameworkError> {
        if !self.world.contains(binding.id) {
            return Err(FrameworkError::MissingView(binding.id));
        }
        if self.world.component_type(binding.id) != Some(&binding.type_id) {
            return Err(FrameworkError::ViewType(binding.id));
        }
        if let Some(component) = binding.retained {
            let register = self
                .components
                .get_by_rust(component.as_ref().type_id())
                .and_then(|entry| entry.register);
            if let Some(register) = register {
                register(self);
            }
            self.views.insert(binding.id, component);
            self.sync_component_lifecycle(binding.id)?;
            if let Some(finish) = binding.finish {
                finish(self, binding.id)?;
            }
        }
        Ok(())
    }

    pub fn install(&mut self, extension: &impl UiExtension) -> Result<(), FrameworkError> {
        let name = extension.name().trim().to_owned();
        if name.is_empty() {
            return Err(FrameworkError::InvalidExtension);
        }
        if self.extensions.contains(&name) {
            return Err(FrameworkError::DuplicateExtension(name));
        }
        let mut registrar = ExtensionRegistrar::default();
        extension.install(&mut registrar)?;
        if let Some(id) = registrar
            .actions
            .keys()
            .find(|id| self.actions.contains_key(*id))
        {
            return Err(FrameworkError::DuplicateAction(id.clone()));
        }
        if let Some(presenter) = registrar
            .presenters
            .iter()
            .find(|presenter| self.world.has_presenter(presenter.name()))
        {
            return Err(FrameworkError::DuplicatePresenter(
                presenter.name().to_owned(),
            ));
        }
        if registrar
            .activations
            .keys()
            .any(|type_id| self.activations.contains_key(type_id))
            || registrar
                .pointers
                .keys()
                .any(|type_id| self.pointer_extensions.contains_key(type_id))
        {
            return Err(FrameworkError::DuplicateActivation);
        }
        self.components.extend(registrar.components)?;
        self.actions.extend(registrar.actions);
        self.activations.extend(registrar.activations);
        self.pointer_extensions.extend(registrar.pointers);
        for presenter in registrar.presenters {
            self.world.register_presenter(presenter)?;
        }
        self.extensions.insert(name);
        Ok(())
    }

    pub fn register_presenter(
        &mut self,
        presenter: Box<dyn TextPresenter>,
    ) -> Result<(), FrameworkError> {
        self.world
            .register_presenter(presenter)
            .map_err(FrameworkError::from)
    }

    /// What a component type brings once, whichever path created its first
    /// node: reprojection and its [`ComponentView::BEHAVIOR`]. Every path
    /// that creates a component goes through this (via
    /// [`Self::stamp_component_type`] or [`Self::install_view`]), or the
    /// type's first node misses its behavior.
    pub(crate) fn register_view_type<C: ComponentView>(&mut self) {
        self.register_view_type_erased(&ViewType::of::<C>());
    }

    /// [`Self::register_view_type`] for a type described by value, so its
    /// map lookups and inserts are compiled once and not per component type.
    #[inline(never)]
    fn register_view_type_erased(&mut self, view: &ViewType) {
        let type_id = view.type_id;
        if self.behaviors.contains_key(&type_id) {
            return;
        }
        self.reprojectors.entry(type_id).or_insert(view.update);
        self.behaviors.insert(type_id, (view.behavior)());
        if let Some(install) = view.hooks {
            install(&mut self.type_hooks);
        }
        if let Some(activation) = view.activation {
            self.activations.entry(type_id).or_insert_with(activation);
        }
    }

    /// Install the view of a node a composite created itself.
    pub(super) fn install_view<C: ComponentView>(&mut self, id: StableNodeId, view: C) {
        self.register_view_type::<C>();
        self.views.insert(id, Box::new(view));
    }

    pub(super) fn stamp_component_type<C: ComponentView>(
        &mut self,
        id: StableNodeId,
        queue: &mut MutationQueue,
    ) {
        self.stamp_component_type_erased(id, queue, &ViewType::of::<C>());
    }

    #[inline(never)]
    pub(super) fn stamp_component_type_erased(
        &mut self,
        id: StableNodeId,
        queue: &mut MutationQueue,
        view: &ViewType,
    ) {
        if let Some(entry) = self.components.get_by_rust(view.type_id) {
            queue.set_component_type(id, Some(entry.id.clone()));
        }
        self.register_view_type_erased(view);

        if view.child_reproject {
            self.child_reproject_views.insert(id, view.reproject);
        }
        if view.metrics_reproject {
            self.metrics_reproject_views.insert(id, view.reproject);
        }
        if view.recipe_reproject {
            self.recipe_reproject_views.insert(id, view.reproject);
        }
        if view.reduced_motion_reproject {
            self.reduced_motion_reproject_views
                .insert(id, view.reproject);
        }
        if view.text_backend_reproject {
            self.text_backend_reproject_views.insert(id, view.reproject);
        }
        if view.hover_tracking {
            self.component_lifecycle.hover_cards.entry(id).or_default();
        }
        self.emitters.entry(view.type_id).or_insert(view.update);
    }

    pub(super) fn allocate_id(&mut self) -> StableNodeId {
        loop {
            let id = StableNodeId::new(self.next_id).expect("allocator never emits zero");
            self.next_id = self
                .next_id
                .checked_add(1)
                .expect("stable ID space exhausted");
            if !self.world.contains(id) && !self.world.is_retired(id) {
                return id;
            }
        }
    }
}

/// What registering and stamping a component type read from it, as plain
/// values: the generic entry points only fill this in.
pub(super) struct ViewType {
    type_id: TypeId,
    update: UpdateErasedFn,
    reproject: ChildReprojectFn,
    behavior: fn() -> hooks::ErasedBehavior,
    hooks: Option<fn(&mut TypeHooks)>,
    activation: Option<fn() -> ActivationFn>,
    child_reproject: bool,
    metrics_reproject: bool,
    recipe_reproject: bool,
    reduced_motion_reproject: bool,
    text_backend_reproject: bool,
    hover_tracking: bool,
}

impl ViewType {
    pub(super) fn of<C: ComponentView>() -> Self {
        Self {
            type_id: TypeId::of::<C>(),
            update: update_erased::<C>,
            reproject: reproject_typed::<C>,
            behavior: hooks::ErasedBehavior::of::<C>,
            hooks: C::BEHAVIOR.hooks,
            activation: C::BEHAVIOR
                .activation
                .map(|_| activation_of::<C> as fn() -> ActivationFn),
            child_reproject: C::wants_child_reproject(),
            metrics_reproject: C::wants_metrics_reproject(),
            recipe_reproject: C::wants_recipe_reproject(),
            reduced_motion_reproject: C::wants_reduced_motion_reproject(),
            text_backend_reproject: C::wants_text_backend_reproject(),
            hover_tracking: C::wants_hover_tracking(),
        }
    }
}

/// The activation a type's [`ComponentView::BEHAVIOR`] names, reached by
/// node id.
fn activation_of<C: ComponentView>() -> ActivationFn {
    let activate = C::BEHAVIOR.activation;
    Arc::new(move |context, id| match activate {
        Some(activate) => activate(context, Entity::from_stable_id(id)),
        None => Ok(false),
    })
}
