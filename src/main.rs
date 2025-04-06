#![feature(try_trait_v2)]

use egui::{RichText, ScrollArea};
use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};
use shellish_parse::ParseOptions;
use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    process::{Command, exit},
    sync::mpsc::{Receiver, Sender, channel},
    time::Duration,
};
use winit::platform::wayland::EventLoopBuilderExtWayland;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Entry {
    name: Option<&'static str>,
    exec: Option<&'static str>,
    icon: Option<&'static str>,
    comment: Option<&'static str>,
}

fn parse_entry<P: AsRef<Path>>(path: P) -> Vec<Entry> {
    print!("{:?}", path.as_ref());
    if path.as_ref().is_dir() {
        let mut result = Vec::new();

        let Ok(dir) = fs::read_dir(path) else {
            return result;
        };

        for p in dir {
            let Ok(p) = p else { continue };
            result.append(&mut parse_entry(p.path()));
        }

        println!(" | recursive");
        return result;
    }

    let Ok(s) = fs::read_to_string(path).map(|s| s.leak()) else {
        println!(" | failed to read");
        return Vec::new();
    };

    let mut name = None;
    let mut icon = None;
    let mut exec = None;
    let mut comment = None;

    for line in s.lines() {
        let Some((key, value)) = line.split_once("=") else {
            continue;
        };
        let value = value.trim();

        match key {
            "Exec" if exec.is_none() => exec = Some(value),
            "Icon" if icon.is_none() => icon = Some(value),
            "Name" if name.is_none() => name = Some(value),
            "Comment" => comment = Some(value),
            "NoDisplay" if value == "true" => {
                println!(" | no display");
                return Vec::new();
            }
            _ => continue,
        };
    }

    if matches!((name, exec, icon, comment), (None, None, None, None)) {
        println!(" | no data");
        return Vec::new();
    }

    println!(" | ok");
    vec![Entry {
        name,
        exec,
        icon,
        comment,
    }]
}

fn get_paths() -> Vec<PathBuf> {
    let home = xdir::data()
        .map(|p| p.join("applications"))
        .take_if(|p| p.exists() && p.is_dir());

    env::var("XDG_DATA_DIRS")
        .iter()
        .flat_map(|s| s.split(':'))
        .map(PathBuf::from)
        .map(|p| p.join("applications"))
        .filter(|p| p.exists())
        .chain(home)
        .chain(["/run/current-system/sw/share/applications".into()])
        .flat_map(fs::read_dir)
        .flatten()
        .flatten()
        .map(|p| p.path())
        .collect()
}
struct State {
    search: String,
    recv: Receiver<(String, Entry)>,
    send: Sender<&'static str>,
    entries: BTreeMap<String, Entry>,
    matcher: SkimMatcherV2,
}

fn main() {
    let (sender, recv) = channel();
    let (command_sender, command_recv) = channel();
    std::thread::spawn(move || {
        for entry in get_paths().into_iter().flat_map(parse_entry) {
            let mut key = String::new();
            if let Some(name) = entry.name {
                key.push_str(name);
            }
            if let Some(comment) = entry.comment {
                if !key.is_empty() {
                    key.push('|');
                }
                key.push_str(comment);
            }
            sender.send((key, entry)).unwrap();
        }
    });

    std::thread::spawn(move || {
        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 240.0]),
            event_loop_builder: Some(Box::new(|event_loop_builder| {
                event_loop_builder.with_any_thread(true);
            })),
            ..Default::default()
        };
        eframe::run_native(
            "My egui App",
            options,
            Box::new(|_| {
                Ok(Box::new(State {
                    search: String::new(),
                    recv,
                    send: command_sender,
                    entries: BTreeMap::new(),
                    matcher: SkimMatcherV2::default().ignore_case().smart_case(),
                }))
            }),
        )
        .unwrap();
    });

    if let Ok(command) = command_recv.recv() {
        let args = shellish_parse::parse(command, ParseOptions::default())
            .unwrap()
            .into_iter()
            .filter(|p| !matches!(&p[..], "%U" | "%u"))
            .collect::<Vec<_>>();
        let exit_status = Command::new(&args[0])
            .args(&args[1..])
            .spawn()
            .unwrap()
            .wait();
        if let Ok(exit_status) = exit_status {
            exit(exit_status.code().unwrap_or_default())
        }
    }
}

impl eframe::App for State {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut i = 0;
        while let Ok((k, t)) = self.recv.try_recv() {
            println!("{t:?}");
            self.entries.insert(k, t);
            i += 1;
            if i >= 60 {
                break;
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            let sl = ui.text_edit_singleline(&mut self.search);
            let open_app = sl.lost_focus();
            sl.request_focus();
            ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show_viewport(ui, |ui, _viewport| {
                    // ui.set_height(16.0 * self.entries.len() as f32);
                    let mut filtered = Vec::new();
                    for (key, entry) in &self.entries {
                        let score = self.matcher.fuzzy_match(key, &self.search);
                        if let Some(score) = score {
                            filtered.push((score, entry));
                        }
                    }

                    filtered.sort_by_key(|(s, _)| -s);

                    for (i, (score, entry)) in filtered.into_iter().enumerate() {
                        ui.horizontal(|ui| {
                            let name = entry.name.unwrap_or("Missing name");

                            ui.label(
                                RichText::new(format!("{:fill$}: ", score, fill = 4)).monospace(),
                            );
                            ui.label(RichText::new(name));
                            if i == 0 && open_app {
                                if let Some(exec) = entry.exec {
                                    self.send.send(exec).unwrap();
                                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                                }
                            }
                        });
                    }
                })
        });
    }
}
