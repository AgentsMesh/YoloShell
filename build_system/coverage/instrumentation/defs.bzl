"""Darwin coverage pages isolated from unrelated globals at every opt level."""

MACOS = "macos"
CONTINUOUS_ENVIRONMENT = "LLVM_PROFILE_CONTINUOUS_MODE"
_ENABLED = "1"
_PAGE_ALIGNMENT = "0x4000"
_DATA_SEGMENT = "__DATA"
_COUNTER_SEGMENT = "__YOLO_PRFC"
_BITMAP_SEGMENT = "__YOLO_PRFB"
_COUNTER_SECTION = "__llvm_prf_cnts"
_BITMAP_SECTION = "__llvm_prf_bits"
_PROFILE_DATA_SECTION = "__llvm_prf_data"
_DIRECT_LINK_ARGUMENT = "-Clink-arg="
_DRIVER_LINK_ARGUMENT = "-Clink-arg=-Wl,"
_SECTION_ALIGNMENT = "-sectalign"
_SECTION_RENAME = "-rename_section"
_ISOLATED_SECTIONS = [
    (_COUNTER_SEGMENT, _COUNTER_SECTION),
    (_BITMAP_SEGMENT, _BITMAP_SECTION),
]
_PROFILE_SECTIONS = _ISOLATED_SECTIONS + [(_DATA_SEGMENT, _PROFILE_DATA_SECTION)]

# MAP_FIXED replaces complete pages, including padding beyond each section.
# Separate segments protect both ends regardless of other sections' ordering.
# Mach-O section boundary symbols follow the linker section rename mapping.
_LINKER_COMMANDS = [
    [_SECTION_RENAME, _DATA_SEGMENT, section, segment, section]
    for segment, section in _ISOLATED_SECTIONS
] + [
    [_SECTION_ALIGNMENT, segment, section, _PAGE_ALIGNMENT]
    for segment, section in _PROFILE_SECTIONS
]

def profile_link_flags(target_os, direct_linker = False):
    if target_os != MACOS:
        return []
    if direct_linker:
        return [
            _DIRECT_LINK_ARGUMENT + argument
            for command in _LINKER_COMMANDS
            for argument in command
        ]
    return [
        _DRIVER_LINK_ARGUMENT + ",".join(command)
        for command in _LINKER_COMMANDS
    ]

def profile_environment(target_os):
    if target_os != MACOS:
        return {}
    return {CONTINUOUS_ENVIRONMENT: _ENABLED}
