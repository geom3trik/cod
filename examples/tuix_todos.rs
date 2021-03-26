
use tuix::*;
use cod::Node;

use std::rc::Rc;

#[derive(Node, Debug, Clone, PartialEq)]
struct Task {
    header: cod::Header,
    description: String,
    completed: bool,
}

#[derive(Node, Clone, Debug)]
struct TodoState {
    header: cod::Header,
    tasks: Vec<cod::Child<Task>>,
}

#[derive(Debug, Clone, PartialEq)]
enum TodoEvent {
    Add(Option<Rc<Task>>),
    Remove,

    Debug,
}

struct TodoApp {
    //state: cod::State<TodoState>,
    index: usize,
    states: Vec<cod::State<TodoState>>,
}

impl TodoApp {
    pub fn new() -> Self {

        let mut states = Vec::new();

        // Add an initial state
        states.push(cod::State::construct(|| {
            let header = cod::Header::new();
            TodoState {
                header: header.clone(),
                tasks: Vec::new(),
            }
        }));

        Self {
            index: 0,
            states,
        }
    }

    pub fn get_current_state(&mut self) -> &mut cod::State<TodoState> {
        &mut self.states[self.index]
    }

    pub fn get_root(&self) -> Rc<TodoState> {
        self.states[self.index].root_ref()
    }
}

impl Widget for TodoApp {
    type Ret = Entity;
    fn on_build(&mut self, state: &mut State, entity: Entity) -> Self::Ret {
        
        state.focused = entity;
        
        entity 
            .set_background_color(state, Color::blue())
            .set_flex_grow(state, 1.0)
    }

    fn on_event(&mut self, state: &mut State, entity: Entity, event: &mut Event) {
        
        // Handle Custom Todo Events
        if let Some(todo_event) = event.message.downcast::<TodoEvent>() {
            match todo_event {
                TodoEvent::Add(task) => {
                    println!("Add a Task");
                    
                    let header = self.get_current_state().header.clone();
                    let mut new_state = self.get_current_state().clone();
                    {
                        new_state.get_mut(new_state.root_ref()).tasks
                            .push(cod::Child::with_parent_header(&header, Task {
                                header: Default::default(),
                                description: "Test".to_string(),
                                completed: false,
                            }));                        
                    }
                    
                    self.states.push(new_state);
                    self.index += 1;
                    // If we had more levels to the hierarchy then maybe send an event with the Rc down to the widget responsible for creating the task widgets
                    // As it happens, that widget is this one so just create the tasks
                    // state.insert_event(Event::new(TodoEvent::Add(self.get_current_state().root().tasks.last().and_then(|r| Some(r.get_ref())))).target(entity).propagate(Propagation::Fall));
                    let new_task = self.get_current_state().root().tasks.last().unwrap().get_ref();
                    TaskWidget::new(new_task.clone()).build(state, entity, |builder| builder);
                }

                TodoEvent::Debug => {
                    println!("{:?}", self.get_current_state().root());
                }

                _=> {}
            }
        }
        
        // Handle Window Events
        if let Some(window_event) = event.message.downcast::<WindowEvent>() {
            match window_event {
                WindowEvent::KeyDown(code, _) => {
                    if *code == Code::KeyA {
                        // Send event to add new task
                        state.insert_event(Event::new(TodoEvent::Add(None)).target(entity));
                    }

                    if *code == Code::KeyD {
                        state.insert_event(Event::new(TodoEvent::Debug).target(entity));
                    }
                }



                _=> {}
            }
        }
    }
}


struct TaskWidget {
    task: Rc<Task>
}

impl TaskWidget {
    pub fn new(task: Rc<Task>) -> Self {
        Self {
            task: task.clone(),
        }
    }
}

impl Widget for TaskWidget {
    type Ret = Entity;
    fn on_build(&mut self, state: &mut State, entity: Entity) -> Self::Ret {
        entity
            .set_flex_basis(state, Length::Pixels(50.0))
            .set_background_color(state, Color::red())
            .set_margin(state, Length::Pixels(5.0))
    }
}



fn main() {
    let app = Application::new(|state, window| {
        window.set_title("Tuix Todos");


        TodoApp::new().build(state, window.entity(), |builder| builder);

    });

    app.run();
}