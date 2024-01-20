use std::collections::HashMap;

pub enum Render {
    LineEdit,
    TextEdit,
    ComboBox,
    Slider,
    SpinBox,
    CheckBox,
    Switch,
}

pub enum Checker {
    Text,
    TextRange,
    Boolean,
    Number,
    NumberRange,
}

pub struct Input {
    id: String,
    description: String,
    value: String,
    reflect: HashMap<String, String>, // description => rendered value
    minimum: Option<f64>,
    maximum: Option<f64>,
    render: Render,
    checker: Checker,
    force_quotes: bool,
}

impl Input {
    pub fn check_value(&self) -> bool {
        match self.checker {
            Checker::Text => true,
            Checker::Boolean => self.value.parse::<bool>().is_ok(),
            Checker::Number => self.value.parse::<f64>().is_ok(),
            Checker::TextRange => self.reflect.contains_key(&self.value),
            Checker::NumberRange => {
                if let Ok(number) = self.value.parse::<f64>() {
                    number >= self.minimum.unwrap_or(0.0) && number <= self.maximum.unwrap_or(100.0)
                } else {
                    false
                }
            }
        }
    }
}
