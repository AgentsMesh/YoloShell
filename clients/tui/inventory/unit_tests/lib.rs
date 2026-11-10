#[path = "../test_support/constants.rs"]
mod constants;
#[path = "../test_support/fixtures.rs"]
mod fixtures;

mod filtering;
mod filtering_display_name;
mod lifecycle_scope;
mod model;
mod pointer_fences;
mod pointer_selection;
#[path = "../test_support/pointer.rs"]
mod pointer_support;
mod pointer_viewport;
mod selection;
mod terminal;
mod title;
mod working_directory;
mod workspace_filter;
