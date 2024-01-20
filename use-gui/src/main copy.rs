use slint::{platform::Clipboard, Model, SharedString};
use std::rc::Rc;

mod input;

slint::include_modules!();

fn main() {
    let main_window = MainWindow::new().unwrap();
    let key_value_pairs = Rc::new(slint::VecModel::<KeyValuePair>::from(vec![]));
    main_window
        .global::<KeyValuePairs>()
        .set_key_value_pairs(key_value_pairs.clone().into());
    main_window.global::<KeyValuePairs>().on_clear({
        let key_value_pairs = key_value_pairs.clone();
        move || {
            key_value_pairs.set_vec(vec![]);
        }
    });
    main_window.global::<KeyValuePairs>().on_add({
        let key_value_pairs = key_value_pairs.clone();
        move || {
            let key_value_pair = KeyValuePair {
                key: "".into(),
                value: "".into(),
            };
            key_value_pairs.insert(0, key_value_pair);
        }
    });
    main_window.global::<KeyValuePairs>().on_remove({
        let key_value_pairs = key_value_pairs.clone();
        move |index| {
            key_value_pairs.remove(index as usize);
        }
    });
    main_window.global::<KeyValuePairs>().on_update({
        let main_window = main_window.as_weak();
        let key_value_pairs = key_value_pairs.clone();
        move || {
            let main_window = main_window.unwrap();
            let mut text: Vec<String> = Vec::new();
            for pair in key_value_pairs.iter() {
                text.push(format!("{} => {}", pair.key.as_str(), pair.value.as_str()));
            }
            let text = text.join("\n");
            main_window.global::<KeyValuePairs>().set_text(text.into());
        }
    });

    let list_of_widget_menifest = Rc::new(slint::VecModel::<WidgetMenifest>::from(vec![]));
    let list_of_widget_name = Rc::new(slint::VecModel::<slint::StandardListViewItem>::from(vec![]));
    let widget_menifest_singleton = main_window.global::<WidgetManifestSingleton>();
    widget_menifest_singleton.on_push_from_clipboard({
        let list_of_menifest = list_of_widget_menifest.clone();
        let list_of_name = list_of_widget_name.clone();
        move || {
            list_of_menifest.push(WidgetMenifest {
                checker: "".into(),
                default_value: "".into(),
                description: "".into(),
                maximum: "".into(),
                minimum: "".into(),
                name: "".into(),
                quotes_option: "".into(),
                reflection: "".into(),
                render: "".into(),
                uuid: "".into(),
            });
            list_of_name.push(slint::StandardListViewItem::from(""));
        }
    });
    widget_menifest_singleton.set_list_of_widget_menifest(list_of_widget_menifest.into());
    widget_menifest_singleton.set_list_of_name(list_of_widget_name.into());
    main_window.run().unwrap();
}
