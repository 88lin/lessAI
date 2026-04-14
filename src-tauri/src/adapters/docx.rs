#[path = "docx/display.rs"]
mod display;
#[path = "docx/model.rs"]
mod model;
#[path = "docx/numbering.rs"]
mod numbering;
#[path = "docx/placeholders.rs"]
mod placeholders;
#[path = "docx/simple.rs"]
mod simple;
#[path = "docx/specials.rs"]
mod specials;
#[path = "docx/styles.rs"]
mod styles;
#[cfg(test)]
#[path = "docx/tests.rs"]
mod tests;

pub use simple::DocxAdapter;
