use cod::{ID, NodeClone, Rc, Weak};
use tuix::*;
use std::cell::RefCell;

use crate::DynUpdateEvent;

#[derive(Debug, PartialEq, Clone)]
pub enum ConfigureObserver {
    RegisterRoot,
    Register(ID),
    Replace(ID),
    Unregister(ID),
    UnregisterEntity,
}

#[derive(Clone)]
pub struct MutationEvent {
    id: ID,
    valid_for: Weak<dyn NodeClone>,
    // TODO: separate facility for FnOnce?
    apply: Rc<RefCell<dyn FnMut(&mut dyn NodeClone)>>,
}

impl MutationEvent {
    pub fn new<T: NodeClone>(node: &Rc<T>, mut apply: impl FnMut(&mut T) + 'static) -> Self {
        Self {
            id: node.header().id(),
            valid_for: Rc::downgrade(node) as Weak<dyn NodeClone>,
            apply: Rc::new(RefCell::new(
                move |node_ref: &mut dyn NodeClone| { apply(cod::downcast_mut(node_ref).unwrap()); }
            )),
        }
    }
}

/// Mutate a cod::Node assuming that there is a MutationManager above this Widget.
pub fn mutate<T: NodeClone>(state: &mut State, entity: Entity, node: &Rc<T>, apply: impl FnMut(&mut T) + 'static) {
    state.insert_event(Event::new(
        MutationEvent::new(node, apply)
    ).propagate(Propagation::Up).target(entity).origin(entity));
}

pub fn configure_observer(state: &mut State, entity: Entity, data: ConfigureObserver) {
    state.insert_event(Event::new(data).propagate(Propagation::Up).target(entity).origin(entity));
}

use std::fmt;
impl fmt::Debug for MutationEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MutationDescription")
            .field("id", &self.id)
            .field("valid_for", &Weak::as_ptr(&self.valid_for))
            .field("apply", &"...")
            .finish()
    }
}
impl PartialEq for MutationEvent {
    fn eq(&self, _other: &Self) -> bool { false }
}

pub struct MutationManager<T: NodeClone + Clone> {
    state: cod::State<T>,
    observers: Vec<Observer>,
}

struct Observer {
    entity: Entity,
    id: ID,
    ptr: Weak<dyn NodeClone>,
    keep: bool,
}

impl<T: NodeClone + Clone> MutationManager<T> {
    pub fn new(state: cod::State<T>) -> Self {
        MutationManager {
            state,
            observers: Default::default(),
        }
    }

    pub fn on_event(&mut self, state: &mut State, event: &mut Event) {
        if let Some(msg) = event.message.downcast() {
            let entity = event.origin;
            if entity == Entity::null() {
                panic!("received ConfigureObserver event without origin");
            }
            match msg {
                ConfigureObserver::RegisterRoot => {
                    let id = self.state.root().header().id();
                    self.add_observer(state, entity, id);
                }
                ConfigureObserver::Register(id) => {
                    self.add_observer(state, entity, *id);
                },
                ConfigureObserver::Replace(id) => {
                    self.remove_entity(entity);
                    self.add_observer(state, entity, *id);
                },
                ConfigureObserver::Unregister(id) => {
                    self.remove_observer(entity, *id);
                },
                ConfigureObserver::UnregisterEntity => {
                    self.remove_entity(entity);
                },
            }
            event.consume();
        }
        if let Some(desc) = event.message.downcast::<MutationEvent>() {
            let node = self.state.ref_from_id(desc.id).expect("mutated node not found in tree");
            // discard vtable pointers at this point, they may originate from different crates (??)
            // correct TypeId is checked later
            if !std::ptr::eq(Rc::as_ptr(&node) as *const (), Weak::as_ptr(&desc.valid_for) as *const ()) {
                panic!("mutation does not apply to current version of node");
            }
            {
                let mut node_mut = self.state.get_mut_dyn(node);
                desc.apply.borrow_mut()(&mut *node_mut);
            }
            // FIXME: optimize
            self.check_all_observers(state, true);
            event.consume();
        }
    }

    fn check_all_observers(&mut self, state: &mut State, animate: bool) {
        let new_state = self.state.clone();
        for Observer { entity, id, ptr, keep } in self.observers.iter_mut() {
            if let Some(new_ref) = new_state.ref_from_id(*id) {
                if Weak::as_ptr(ptr) != Rc::as_ptr(&new_ref) {
                    *ptr = Rc::downgrade(&new_ref);
                    state.insert_event(Event::new(DynUpdateEvent::Update(new_ref, animate)).direct(*entity));
                }
                *keep = true;
            } else {
                state.insert_event(Event::new(DynUpdateEvent::Remove(*id, animate)).direct(*entity));
                *keep = false;
            }
        }
        self.observers.retain(|t| t.keep);
    }

    fn add_observer(&mut self, state: &mut State, entity: Entity, id: ID) {
        if let Some(node) = self.state.ref_from_id(id) {
            self.observers.push(Observer {
                entity,
                id,
                ptr: Rc::downgrade(&node),
                keep: true,
            });
            state.insert_event(Event::new(DynUpdateEvent::Update(node, false)).direct(entity));
        } else {
            state.insert_event(Event::new(DynUpdateEvent::Remove(id, false)).direct(entity));
        }
    }

    fn remove_observer(&mut self, entity: Entity, id: ID) {
        self.observers.retain(|t| t.entity != entity || t.id != id);
    }

    fn remove_entity(&mut self, entity: Entity) {
        self.observers.retain(|t| t.entity != entity);
    }

    pub fn state(&self) -> &cod::State<T> {
        &self.state
    }

    pub fn clone_state(&self) -> cod::State<T> {
        self.state.clone()
    }

    pub fn replace_state(&mut self, state: &mut State, replacement: cod::State<T>) {
        self.state = replacement;
        self.check_all_observers(state, false);
    }
}
