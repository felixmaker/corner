use std::{path::PathBuf, time::Duration};

use fltk::{prelude::*, *};
use fltk_table::*;
use srtlib::{Subtitle, Timestamp};

mod ui {
    pub mod mainform {
        fl2rust_macro::include_ui!("./src/ui/mainform.fl");
    }

    pub mod combine_dialog {
        fl2rust_macro::include_ui!("./src/ui/combine_dialog.fl");
    }
}

mod ffopt;

#[derive(Clone)]
enum Message {
    LoadAudio,
    LoadSubtitle,
    DetectSilence(i32),
    CopyToTTSMaker,
    ShowConbineDialog,
    StartCombine(ItemType, ItemType),
    SetTableRowLength,
}

#[derive(Clone, Debug)]
#[repr(i32)]
enum ItemType {
    Audio = 0,
    Subtitle = 1,
}

impl From<i32> for ItemType {
    fn from(value: i32) -> Self {
        match value {
            0 => ItemType::Subtitle,
            1 => ItemType::Audio,
            _ => ItemType::Subtitle,
        }
    }
}

fn set_table_opt(table: &mut SmartTable, rows: i32, text_col_width: i32) {
    if let Some(input) = table.input() {
        input.hide();
    }

    table.set_opts(TableOpts {
        rows,
        cols: 3,
        editable: true,
        cell_align: enums::Align::Left,
        cell_padding: 5,
        header_align: enums::Align::Left,
        ..Default::default()
    });

    table.set_col_header_value(0, "Start Time");
    table.set_col_header_value(1, "End Time");
    table.set_col_header_value(2, "Text");
    table.set_col_width(0, 110);
    table.set_col_width(1, 110);
    table.set_col_width(2, text_col_width);

    table.redraw();
}

fn main() {
    let app = app::App::default();
    let (sender, receiver) = fltk::app::channel();

    // combine dialog
    let mut dialog = ui::combine_dialog::UserInterface::make_window();

    dialog.btn_cancel.set_callback({
        let mut window = dialog.window.clone();
        move |_| window.hide()
    });

    dialog.btn_confirm.set_callback({
        let mut window = dialog.window.clone();
        let choice_start = dialog.choice_start.clone();
        let choice_duration = dialog.choice_duration.clone();
        let sender = sender.clone();
        move |_| {
            let choice_start = choice_start.value();
            let choice_duration = choice_duration.value();
            sender.send(Message::StartCombine(
                choice_start.into(),
                choice_duration.into(),
            ));
            window.hide();
        }
    });

    let mut ui = ui::mainform::UserInterface::make_window();

    // Load smart table
    ui.table_parent.begin();
    let mut table = SmartTable::default().size_of_parent().center_of_parent();

    set_table_opt(&mut table, 1, 110);
    ui.table_parent.end();

    ui.btn_load_audio.emit(sender.clone(), Message::LoadAudio);
    ui.btn_load_subtitle
        .emit(sender.clone(), Message::LoadSubtitle);
    ui.btn_copy.emit(sender.clone(), Message::CopyToTTSMaker);
    ui.btn_combine
        .emit(sender.clone(), Message::ShowConbineDialog);
    ui.btn_add_row
        .emit(sender.clone(), Message::SetTableRowLength);

    while app.wait() {
        if let Some(message) = receiver.recv() {
            match message {
                Message::LoadAudio => {
                    let filename = dialog::file_chooser("Choose Audio File", "*.mp3", ".", false);
                    ui.input_file.set_value(&filename.unwrap_or("".to_owned()));

                    let duration = ui.input_duration.value() as i32;
                    sender.send(Message::DetectSilence(duration));
                }

                Message::DetectSilence(duration) => {
                    let audio = PathBuf::from(ui.input_file.value());
                    let duration = Duration::from_millis(duration as u64);

                    match ffopt::detect_silence(audio, duration) {
                        Ok(silence) => {
                            let silence_count = silence.len();
                            let audio_count = (silence_count + 1) as i32;
                            ui.box_count.set_label(audio_count.to_string().as_str());
                        }
                        Err(err) => {
                            println!("{}", err)
                        }
                    }
                }

                Message::LoadSubtitle => {
                    let filename =
                        dialog::file_chooser("Choose Subtitle File", "*.srt", ".", false);
                    if filename.is_none() {
                        continue;
                    }
                    match srtlib::Subtitles::parse_from_file(filename.unwrap(), None) {
                        Ok(subtitles) => {
                            let rows = subtitles.len() as i32;
                            set_table_opt(&mut table, rows, 400);

                            for row in 0..rows {
                                let subtitle = &subtitles[row as usize];
                                table.set_cell_value(
                                    row,
                                    0,
                                    subtitle.start_time.to_string().as_str(),
                                );
                                table.set_cell_value(
                                    row,
                                    1,
                                    subtitle.end_time.to_string().as_str(),
                                );
                                table.set_cell_value(row, 2, subtitle.text.to_string().as_str());
                            }
                        }
                        Err(error) => {
                            dialog::message_default(error.to_string().as_str());
                        }
                    }
                }

                Message::CopyToTTSMaker => {
                    let data = table.data();
                    let mut result = Vec::new();
                    for row in data {
                        if let Some(text) = row.get(2).map(|x| x.to_owned()) {
                            result.push(text.to_string());
                        }
                    }
                    let result = result.join("\n");
                    app::copy(&result);
                }

                Message::ShowConbineDialog => {
                    dialog.window.show();
                }

                Message::StartCombine(combine_start, combine_duration) => {
                    // Main work here
                    let audio = ui.input_file.value();
                    let audio = PathBuf::from(audio);
                    let duration = ui.input_duration.value();
                    let duration = Duration::from_millis(duration as u64);

                    let temp_dir = PathBuf::from("./temp");
                    let audio_pieces = ffopt::get_audio_pieces(&audio, duration).unwrap();

                    let sub_data = table.data();

                    let audio_len = audio_pieces.len();
                    let sub_len = sub_data.len();

                    if audio_len != sub_len {
                        let choice = dialog::choice2_default(
                            "Audio and text length does not match. Are you sure to continue?",
                            "Yes",
                            "No",
                            "",
                        )
                        .unwrap_or(1);

                        if choice != 0 {
                            continue;
                        }
                    }

                    let result_count = usize::min(audio_len, sub_len);
                    let audio_info =
                        ffopt::cut_audio2(&audio, audio_pieces.as_slice(), &temp_dir).unwrap();
                    let sub_info = {
                        let mut sub_info = Vec::new();
                        for item in sub_data {
                            let (start, end, text) = (&item[0], &item[1], &item[2]);
                            let start = ffopt::parse_timestamp(&start).unwrap();
                            let end = ffopt::parse_timestamp(&end).unwrap();
                            sub_info.push((start, end - start, text.to_owned()))
                        }
                        sub_info
                    };

                    let mut audio_joininfo = Vec::new();
                    let mut subs = srtlib::Subtitles::new();
                    for i in 0..result_count {
                        let (audio_start, audio_duration, audio) = &audio_info[i];
                        let (sub_start, sub_duration, text) = &sub_info[i];

                        let start = match combine_start {
                            ItemType::Audio => audio_start.to_owned(),
                            ItemType::Subtitle => sub_start.to_owned(),
                        };

                        let duration = match combine_duration {
                            ItemType::Audio => audio_duration.to_owned(),
                            ItemType::Subtitle => sub_duration.to_owned(),
                        };

                        audio_joininfo.push((start, audio.to_owned()));

                        let mut start_time = Timestamp::new(0, 0, 0, 0);
                        start_time.add_milliseconds(start.as_millis() as i32);

                        let mut end_time = Timestamp::new(0, 0, 0, 0);
                        end_time.add_milliseconds((start + duration).as_millis() as i32);

                        subs.push(Subtitle {
                            num: i,
                            start_time,
                            end_time,
                            text: text.to_owned(),
                        })
                    }

                    ffopt::join_audios(audio_joininfo.as_slice(), &PathBuf::from("./result.mp3"))
                        .unwrap();
                    subs.write_to_file("./result.srt", None).unwrap();
                }

                Message::SetTableRowLength => {
                    if let Some(rows) = dialog::input_default("How much rows to add in table?", "1")
                        .map(|x| x.parse::<i32>().unwrap_or(1))
                    {
                        let text_col_width = table.col_width(2);
                        set_table_opt(&mut table, rows, text_col_width);
                    }
                }
            }
        }
    }
}
