"""Check the final Mach-O format beside real native and Scoop exception tests."""

import struct
import sys
from pathlib import Path


def check(path: Path) -> None:
    data = path.read_bytes()
    assert struct.unpack_from("<I", data)[0] == 0xFEEDFACF, path
    command_count = struct.unpack_from("<I", data, 16)[0]
    offset = 32
    sections = {}
    tools = []
    for _ in range(command_count):
        command, size = struct.unpack_from("<II", data, offset)
        if command == 0x19:  # LC_SEGMENT_64
            section_count = struct.unpack_from("<I", data, offset + 64)[0]
            for index in range(section_count):
                section = offset + 72 + index * 80
                name = data[section : section + 16].rstrip(b"\0")
                length, location = struct.unpack_from("<QI", data, section + 40)
                sections[name] = (location, length)
        elif command == 0x32:  # LC_BUILD_VERSION
            tool_count = struct.unpack_from("<I", data, offset + 20)[0]
            tools.extend(
                struct.unpack_from("<I", data, offset + 24 + index * 8)[0]
                for index in range(tool_count)
            )
        offset += size

    assert tools == [4], (path, tools)  # TOOL_LLD
    unwind_offset, unwind_size = sections[b"__unwind_info"]
    assert unwind_size >= 28, path
    header = struct.unpack_from("<7I", data, unwind_offset)
    assert header[0] == 1, (path, header)
    # All three native handlers run. A compact Scoop personality would be a fourth.
    assert header[4] == 3, (path, header)
    assert sections[b"__eh_frame"][1] > 0, path


for argument in sys.argv[1:]:
    check(Path(argument))
