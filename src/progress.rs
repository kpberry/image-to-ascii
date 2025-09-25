use indicatif::{ProgressBar, ProgressStyle};

pub fn default_progress_bar(label: &str, n_items: usize) -> ProgressBar {
    let progress_template = &format!(
        "[{{wide_bar}}] {}: {{pos}}/{{len}} Time: ({{elapsed}}/{{duration}})",
        label
    );
    let progress = ProgressBar::new(n_items as u64);
    progress.set_style(ProgressStyle::default_bar().template(progress_template).unwrap());
    progress
}
