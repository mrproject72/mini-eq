//! AutoEq import dialog with curve preview.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use gtk4::cairo::Context;
use gtk4::prelude::*;

use crate::autoeq::{AutoEqEntry, download_autoeq_preset, parse_apo_file, search_autoeq_entries};
use crate::core::{
    EQ_FREQUENCY_MAX_HZ, EQ_FREQUENCY_MIN_HZ, EQ_GAIN_MAX_DB, EQ_GAIN_MIN_DB, SAMPLE_RATE,
    total_response_db_at_frequencies,
};

/// AutoEq import dialog.
pub struct AutoEqDialog {
    pub dialog: gtk4::Dialog,
    pub search_entry: gtk4::SearchEntry,
    pub results_list: gtk4::ListBox,
    pub status_label: gtk4::Label,
    pub import_button: gtk4::Button,
    pub preview_area: gtk4::DrawingArea,
    pub preview_bands: Rc<RefCell<Vec<crate::core::EqBand>>>,
    pub preview_preamp: Rc<RefCell<f64>>,
    _entries: Rc<RefCell<Vec<AutoEqEntry>>>,
    _cache_dir: PathBuf,
}

impl AutoEqDialog {
    pub fn new(parent: &impl IsA<gtk4::Window>, cache_dir: PathBuf) -> Self {
        let dialog = gtk4::Dialog::new();
        dialog.set_transient_for(Some(parent));
        dialog.set_title(Some("Import from AutoEq"));
        dialog.set_default_size(640, 520);

        let content = dialog.content_area();
        content.set_margin_top(18);
        content.set_margin_bottom(18);
        content.set_margin_start(18);
        content.set_margin_end(18);
        content.set_spacing(12);

        let search_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        let search_entry = gtk4::SearchEntry::new();
        search_entry.set_hexpand(true);
        search_entry.set_placeholder_text(Some("Search headphones..."));
        search_row.append(&search_entry);

        let refresh_button = gtk4::Button::from_icon_name("view-refresh-symbolic");
        refresh_button.set_tooltip_text(Some("Refresh AutoEq Profiles"));
        search_row.append(&refresh_button);
        content.append(&search_row);

        let status_label = gtk4::Label::new(Some("Loading AutoEq profiles..."));
        status_label.set_halign(gtk4::Align::Start);
        status_label.add_css_class("dim-label");
        content.append(&status_label);

        let results_list = gtk4::ListBox::new();
        results_list.set_selection_mode(gtk4::SelectionMode::Single);
        results_list.add_css_class("boxed-list");

        let scrolled = gtk4::ScrolledWindow::new();
        scrolled.set_policy(gtk4::PolicyType::Automatic, gtk4::PolicyType::Automatic);
        scrolled.set_min_content_height(200);
        scrolled.set_child(Some(&results_list));
        content.append(&scrolled);

        let preview_area = gtk4::DrawingArea::new();
        preview_area.set_size_request(-1, 140);
        preview_area.set_vexpand(true);
        content.append(&preview_area);

        let import_button = gtk4::Button::with_label("Import Selected");
        import_button.set_sensitive(false);
        content.append(&import_button);

        let entries = Rc::new(RefCell::new(Vec::new()));
        let preview_bands = Rc::new(RefCell::new(Vec::new()));
        let preview_preamp = Rc::new(RefCell::new(0.0));

        let entries_for_search = entries.clone();
        let results_list_for_search = results_list.clone();
        let status_for_search = status_label.clone();
        let preview_bands_for_search = preview_bands.clone();
        let preview_preamp_for_search = preview_preamp.clone();
        let preview_area_for_search = preview_area.clone();
        search_entry.connect_search_changed(move |entry| {
            let query = entry.text();
            let current = entries_for_search.borrow();
            let results = search_autoeq_entries(&current, &query, 50);
            drop(current);

            while let Some(child) = results_list_for_search.first_child() {
                results_list_for_search.remove(&child);
            }

            if results.is_empty() {
                let text = if query.is_empty() {
                    "Enter a search query"
                } else {
                    "No results found"
                };
                status_for_search.set_text(text);
                *preview_bands_for_search.borrow_mut() = Vec::new();
                *preview_preamp_for_search.borrow_mut() = 0.0;
                preview_area_for_search.queue_draw();
            } else {
                status_for_search.set_text(&format!("{} results", results.len()));
                for entry in &results {
                    let row = gtk4::ListBoxRow::new();
                    let label = gtk4::Label::new(Some(&entry.name));
                    label.set_halign(gtk4::Align::Start);
                    row.set_child(Some(&label));
                    results_list_for_search.append(&row);
                }
            }
        });

        let entries_for_refresh = entries.clone();
        let results_list_for_refresh = results_list.clone();
        let status_for_refresh = status_label.clone();
        let cache_dir_for_refresh = cache_dir.clone();
        refresh_button.connect_clicked(move |_| {
            status_for_refresh.set_text("Refreshing...");
            let cache_dir = cache_dir_for_refresh.clone();
            let entries_for_refresh = entries_for_refresh.clone();
            let results_list_for_refresh = results_list_for_refresh.clone();
            let status_for_refresh = status_for_refresh.clone();

            glib::MainContext::default().spawn_local(async move {
                match crate::autoeq::load_autoeq_entries(&cache_dir).await {
                    Ok(new_entries) => {
                        *entries_for_refresh.borrow_mut() = new_entries;
                        while let Some(child) = results_list_for_refresh.first_child() {
                            results_list_for_refresh.remove(&child);
                        }
                        status_for_refresh.set_text(&format!(
                            "Loaded {} profiles",
                            entries_for_refresh.borrow().len()
                        ));
                    }
                    Err(e) => {
                        status_for_refresh.set_text(&format!("Error: {}", e));
                    }
                }
            });
        });

        let entries_for_select = entries.clone();
        let preview_bands_for_select = preview_bands.clone();
        let preview_preamp_for_select = preview_preamp.clone();
        let preview_area_for_select = preview_area.clone();
        let status_for_select = status_label.clone();
        let cache_dir_for_select = cache_dir.clone();
        results_list.connect_selected_rows_changed(move |list| {
            if let Some(row) = list.selected_row() {
                if let Some(child) = row.child() {
                    if let Some(label) = child.downcast_ref::<gtk4::Label>() {
                        let name = label.label();
                        let entries = entries_for_select.borrow();
                        if let Some(entry) = entries.iter().find(|e| e.name == name) {
                            status_for_select.set_text("Loading curve preview...");
                            let entry = entry.clone();
                            let cache_dir = cache_dir_for_select.clone();
                            let preview_bands = preview_bands_for_select.clone();
                            let preview_preamp = preview_preamp_for_select.clone();
                            let preview_area = preview_area_for_select.clone();
                            let status = status_for_select.clone();
                            glib::MainContext::default().spawn_local(async move {
                                match download_autoeq_preset(&entry, &cache_dir, SAMPLE_RATE).await
                                {
                                    Ok(preset) => match parse_apo_file(&preset.path) {
                                        Ok((preamp, bands)) => {
                                            status.set_text(&format!(
                                                "{} — {} bands",
                                                entry.name,
                                                bands.len()
                                            ));
                                            *preview_bands.borrow_mut() = bands;
                                            *preview_preamp.borrow_mut() = preamp;
                                            preview_area.queue_draw();
                                        }
                                        Err(e) => {
                                            status.set_text(&format!("Parse error: {}", e));
                                        }
                                    },
                                    Err(e) => {
                                        status.set_text(&format!("Download error: {}", e));
                                    }
                                }
                            });
                        }
                    }
                }
            }
        });

        let results_list_for_import = results_list.clone();
        import_button.connect_clicked(move |_| {
            if let Some(row) = results_list_for_import.selected_row() {
                if let Some(child) = row.child() {
                    if let Some(label) = child.downcast_ref::<gtk4::Label>() {
                        let name = label.label();
                        println!("Importing AutoEq preset: {}", name);
                    }
                }
            }
        });

        results_list.connect_selected_rows_changed({
            let import_button = import_button.clone();
            move |list| {
                import_button.set_sensitive(list.selected_row().is_some());
            }
        });

        let preview_bands_for_draw = preview_bands.clone();
        let preview_preamp_for_draw = preview_preamp.clone();
        preview_area.set_draw_func(move |_area, ctx, width, height| {
            let bands = preview_bands_for_draw.borrow();
            let preamp = *preview_preamp_for_draw.borrow();
            if bands.is_empty() {
                ctx.set_source_rgb(0.08, 0.08, 0.08);
                let _ = ctx.paint();
                return;
            }
            draw_autoeq_preview(ctx, width, height, preamp, &bands);
        });

        dialog.present();
        Self {
            dialog,
            search_entry,
            results_list,
            status_label,
            import_button,
            preview_area,
            preview_bands,
            preview_preamp,
            _entries: entries,
            _cache_dir: cache_dir,
        }
    }

    pub fn show(&self) {
        self.dialog.present();
    }
}

fn draw_autoeq_preview(
    ctx: &Context,
    width: i32,
    height: i32,
    preamp_db: f64,
    bands: &[crate::core::EqBand],
) {
    let width = width as f64;
    let height = height as f64;

    ctx.set_source_rgb(0.08, 0.08, 0.08);
    let _ = ctx.paint();

    let center_y = height / 2.0;
    let gain_range = EQ_GAIN_MAX_DB - EQ_GAIN_MIN_DB;

    ctx.set_source_rgba(0.2, 0.2, 0.2, 0.5);
    ctx.set_line_width(0.5);
    for db in [-20, -10, 0, 10, 20].iter() {
        let y = center_y - (*db as f64 / gain_range) * center_y;
        ctx.move_to(0.0, y);
        ctx.line_to(width, y);
        let _ = ctx.stroke();
    }

    for &freq in [20, 50, 100, 200, 500, 1000, 2000, 5000, 10000, 20000].iter() {
        let x = ((freq as f64).log10() - EQ_FREQUENCY_MIN_HZ.log10())
            / (EQ_FREQUENCY_MAX_HZ.log10() - EQ_FREQUENCY_MIN_HZ.log10())
            * width;
        ctx.move_to(x, 0.0);
        ctx.line_to(x, height);
        let _ = ctx.stroke();
    }

    ctx.set_source_rgba(0.4, 0.4, 0.4, 0.8);
    ctx.set_line_width(1.0);
    ctx.move_to(0.0, center_y);
    ctx.line_to(width, center_y);
    let _ = ctx.stroke();

    let num_points = width as usize;
    let mut frequencies = Vec::with_capacity(num_points);
    for i in 0..num_points {
        let freq = EQ_FREQUENCY_MIN_HZ
            * (EQ_FREQUENCY_MAX_HZ / EQ_FREQUENCY_MIN_HZ).powf(i as f64 / num_points as f64);
        frequencies.push(freq);
    }

    let response = total_response_db_at_frequencies(bands, preamp_db, SAMPLE_RATE, &frequencies);

    ctx.set_source_rgb(0.0, 0.8, 0.0);
    ctx.set_line_width(2.0);
    ctx.move_to(0.0, center_y);
    for (i, &db) in response.iter().enumerate() {
        let y = center_y - (db / gain_range) * center_y;
        ctx.line_to(i as f64, y);
    }
    let _ = ctx.stroke();
}
