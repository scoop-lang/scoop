use super::*;

pub(super) const FILE_NAME: &str = "objects.rsp";

pub(super) fn write_paths(directory: &Path, paths: &[PathBuf]) -> Result<(), LinkError> {
    write_arguments(directory, paths.iter().map(|path| path.as_os_str()))
}

fn write_arguments<'argument>(
    directory: &Path,
    arguments: impl IntoIterator<Item = &'argument std::ffi::OsStr>,
) -> Result<(), LinkError> {
    let mut contents = Vec::new();
    for argument in arguments {
        contents.push(b'"');
        for byte in argument.as_encoded_bytes() {
            if matches!(byte, b'\\' | b'"') {
                contents.push(b'\\');
            }
            contents.push(*byte);
        }
        contents.extend_from_slice(b"\"\n");
    }
    write_new(&directory.join(FILE_NAME), &contents)
}

pub(super) fn write_pe_arguments(
    directory: &Path,
    arguments: &[std::ffi::OsString],
    microsoft: bool,
) -> Result<(), LinkError> {
    if !cfg!(windows) && !microsoft {
        return write_arguments(
            directory,
            arguments.iter().map(|argument| argument.as_os_str()),
        );
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        let mut contents = vec![0xff, 0xfe];
        for argument in arguments {
            let argument: Vec<_> = argument.encode_wide().collect();
            for unit in windows_quote(&argument) {
                contents.extend_from_slice(&unit.to_le_bytes());
            }
        }
        write_new(&directory.join(FILE_NAME), &contents)
    }
    #[cfg(not(windows))]
    {
        let contents: Vec<u8> = arguments
            .iter()
            .flat_map(|argument| windows_quote(argument.as_encoded_bytes()))
            .collect();
        write_new(&directory.join(FILE_NAME), &contents)
    }
}

fn windows_quote<T: Copy + Eq + From<u8>>(argument: &[T]) -> Vec<T> {
    let slash = T::from(b'\\');
    let quote = T::from(b'"');
    let mut result = vec![quote];
    let mut slashes = 0;
    for value in argument {
        if *value == slash {
            slashes += 1;
            continue;
        }
        result.extend(std::iter::repeat_n(
            slash,
            if *value == quote {
                slashes * 2 + 1
            } else {
                slashes
            },
        ));
        result.push(*value);
        slashes = 0;
    }
    result.extend(std::iter::repeat_n(slash, slashes * 2));
    result.extend([quote, T::from(b'\n')]);
    result
}
