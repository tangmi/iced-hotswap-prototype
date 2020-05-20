use serde::{Deserialize, Serialize};

#[cfg(test)]
#[test]
fn print_sample_iml() {
    println!(
        "{}",
        ron::ser::to_string_pretty(
            &Root {
                root: Element::Column(vec![
                    Box::new(Element::Text("hello".to_owned())),
                    Box::new(Element::Text("world".to_owned())),
                    Box::new(Element::Button {
                        content: Box::new(Element::Text("button".to_owned())),
                        callback_name: "Testaroo".to_owned(),
                    }),
                    Box::new(Element::Slider {
                        value_name: "test".to_owned(),
                        range: 0.0..=100.0,
                        callback_name: "TestarooTwo".to_owned()
                    })
                ]),
            },
            ron::ser::PrettyConfig {
                depth_limit: usize::max_value(),
                enumerate_arrays: false,
                indentor: "    ".to_owned(),
                new_line: "\n".to_owned(),
                separate_tuple_members: true,
            },
        )
        .unwrap()
    );
}

#[derive(Debug, Serialize, Deserialize)]
struct Root {
    pub root: Element,
}

#[derive(Debug, Serialize, Deserialize)]
enum Element {
    Column(Vec<Box<Element>>),
    Row(Vec<Box<Element>>),
    Text(String),
    Button {
        content: Box<Element>,
        callback_name: String,
    },
    Slider {
        /// Handlebars-like path to range value?
        value_name: String,
        range: std::ops::RangeInclusive<f32>,
        callback_name: String,
    },
}

/// Type to:
///  - load a templated layout file
///  - resolve the template with user state
///  - load the resolved template into memory
///  - create ephemeral state for widgets (e.g. button::State)
///  - render the in-memory representation as an iced::Element tree
///
/// type S is user-defined state used to resolve the templates as well as in the tree itself
pub struct ImlFile<S: Serialize> {
    templated_source: String,
    layout: Option<Root>,

    user_state: S,
    widget_states: std::cell::RefCell<std::collections::HashMap<String, Box<dyn std::any::Any>>>,
}

impl<S: Serialize> ImlFile<S> {
    pub fn load_iml(
        mut reader: impl std::io::Read,
        initial_user_state: S,
    ) -> std::io::Result<Self> {
        let mut templated_source = String::new();
        reader.read_to_string(&mut templated_source)?;

        Ok(Self {
            templated_source,
            layout: None,

            user_state: initial_user_state,
            widget_states: std::cell::RefCell::new(std::collections::HashMap::new()),
        })
    }

    pub fn state(&mut self) -> &mut S {
        &mut self.user_state
    }

    pub fn view<'a, M: 'static + Copy + Clone + Deserialize<'a>>(
        &'a mut self,
    ) -> iced::Element<'a, M> {
        const TEMPLATE_NAME: &str = "layout_template";
        let mut handlebars = handlebars::Handlebars::new();

        handlebars
            .register_template_string(TEMPLATE_NAME, &self.templated_source)
            .expect("template compile succeeded");

        let source = handlebars
            .render(TEMPLATE_NAME, &self.user_state)
            .expect("template render succeeded");

        self.layout = Some(ron::de::from_str(&source).expect("resolved templeate parsed"));

        Self::get_iced_element(
            &self.user_state,
            &self.widget_states,
            &self.layout.as_ref().expect("just set this field").root,
            "root".to_owned(),
        )
    }

    fn get_iced_element<'a, M: 'static + Copy + Clone + Deserialize<'a>>(
        user_state: &S,
        widget_states: &'a std::cell::RefCell<
            std::collections::HashMap<String, Box<dyn std::any::Any>>,
        >,
        element: &'a Element,
        id: String,
    ) -> iced::Element<'a, M> {
        // TODO find a way to discover and handle custom widgets?
        match element {
            Element::Column(children) => {
                // Explicit for loop b/c I don't know how to make lifetimes work with closures
                let mut children_elements = Vec::new();
                for (i, child) in children.iter().enumerate() {
                    children_elements.push(
                        Self::get_iced_element(
                            user_state,
                            widget_states,
                            child,
                            format!("{}.column.children[{}]", &id, i),
                        )
                        .into(),
                    );
                }

                iced::widget::Column::with_children(children_elements).into()
            }

            Element::Row(children) => {
                let mut children_elements = Vec::new();
                for (i, child) in children.iter().enumerate() {
                    children_elements.push(
                        Self::get_iced_element(
                            user_state,
                            widget_states,
                            child,
                            format!("{}.row.children[{}]", &id, i),
                        )
                        .into(),
                    );
                }

                iced::widget::Row::with_children(children_elements).into()
            }

            Element::Text(s) => iced::widget::Text::new(s).into(),

            Element::Button {
                content,
                callback_name,
            } => {
                let id = format!("{}.button", id);
                iced::button::Button::new(
                    Self::create_or_get_widget_state(widget_states, &id),
                    Self::get_iced_element(user_state, widget_states, content, id),
                )
                .on_press(ron::de::from_str(callback_name).unwrap())
                .into()
            }

            Element::Slider {
                value_name,
                range,
                callback_name,
            } => {
                let id = format!("{}.slider", id);
                iced::slider::Slider::new(
                    Self::create_or_get_widget_state(widget_states, &id),
                    range.clone(),
                    Self::get_value_from_user_state(user_state, value_name)
                        .as_f64()
                        .unwrap() as f32,
                    {
                        let name = callback_name.clone(); // needed for lifetime reasons???
                        move |val| {
                            let v = format!("{}({})", name, val);
                            ron::de::from_str(unsafe {
                                // i need lifetime help!! this is a bad idea!!!!
                                std::mem::transmute(v.as_str())
                            })
                            .unwrap()
                        }
                    },
                )
                .into()
            }
        }
    }

    /// roundtrip the state through json to get it as a dynamic map
    fn get_value_from_user_state(user_state: &S, key: &String) -> serde_json::value::Value {
        // serde_json::value::Value::Number(serde_json::Number::from_f64(1.0).unwrap())

        let serialized = serde_json::ser::to_string(user_state).unwrap();
        let root: serde_json::value::Value = serde_json::de::from_str(&serialized).unwrap();

        // todo allow multi-level indexing (i.e. "field.inner_array[9].field")
        root.get(key)
            .expect(&format!("key {} wasn't found in user state struct", key))
            .clone()
    }

    fn create_or_get_widget_state<'a, WidgetState: Default>(
        states: &'a std::cell::RefCell<std::collections::HashMap<String, Box<dyn std::any::Any>>>,
        id: &str,
    ) -> &'a mut WidgetState {
        let mut state = std::cell::RefMut::map(states.borrow_mut(), |state| {
            state
                .entry(id.to_owned())
                .or_insert(Box::new(iced::widget::button::State::new()))
                .downcast_mut::<iced::widget::button::State>()
                .expect("state should have the right type")
        });

        unsafe {
            use std::ops::DerefMut;
            // very bad! hack! force the lifetime to work
            std::mem::transmute(state.deref_mut())
        }
    }
}
