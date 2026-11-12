//! Bounded inspection of the final Mach-O image used by coverage tests.

#[path = "macho_constants.rs"]
mod constants;
#[path = "macho_geometry.rs"]
mod geometry;
#[path = "macho_parser.rs"]
mod parser;
#[path = "macho_reader.rs"]
mod reader;
#[path = "macho_sections.rs"]
mod sections;
#[path = "macho_verify.rs"]
mod verify;

pub use parser::inspect;
pub use verify::verify_profile_layout;

#[derive(Clone, Debug)]
pub struct Image {
    pub segments: Vec<Segment>,
    pub file_size: u64,
}

#[derive(Clone, Debug)]
pub struct Segment {
    pub name: String,
    pub address: u64,
    pub size: u64,
    pub file_offset: u64,
    pub file_size: u64,
    pub max_protection: u32,
    pub initial_protection: u32,
    pub flags: u32,
    pub sections: Vec<Section>,
}

#[derive(Clone, Debug)]
pub struct Section {
    pub name: String,
    pub segment: String,
    pub address: u64,
    pub size: u64,
    pub file_offset: u64,
    pub flags: u32,
    pub alignment: u32,
}
