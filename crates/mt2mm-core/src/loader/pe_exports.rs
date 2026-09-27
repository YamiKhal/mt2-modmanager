use anyhow::{bail, Context, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeExport {
    pub name: String,
    pub forward: Option<String>,
}

struct Section {
    virtual_address: u32,
    virtual_size: u32,
    raw_offset: u32,
    raw_size: u32,
}

const PE32_PLUS_MAGIC: u16 = 0x20b;
pub const MACHINE_X64: u16 = 0x8664;
const PE32_MAGIC: u16 = 0x10b;
const MAX_EXPORTS: u32 = 100_000;


fn read_u16(bytes: &[u8], offset: usize) -> Result<u16> {
    let slice = bytes.get(offset..offset + 2).context("file ends inside its headers")?;

    Ok(u16::from_le_bytes([slice[0], slice[1]]))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32> {
    let slice = bytes.get(offset..offset + 4).context("file ends inside its headers")?;

    Ok(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

fn read_c_string(bytes: &[u8], offset: usize) -> Result<String> {
    let tail = bytes.get(offset..).context("name points outside the file")?;
    let length = tail.iter().position(|&byte| byte == 0).context("name has no end")?;

    Ok(String::from_utf8_lossy(&tail[..length]).into_owned())
}


fn read_sections(bytes: &[u8], table_offset: usize, count: u16) -> Result<Vec<Section>> {
    let mut sections = Vec::with_capacity(count as usize);

    for index in 0..count as usize {
        let entry = table_offset + index * 40;

        sections.push(Section {
            virtual_size: read_u32(bytes, entry + 8)?,
            virtual_address: read_u32(bytes, entry + 12)?,
            raw_size: read_u32(bytes, entry + 16)?,
            raw_offset: read_u32(bytes, entry + 20)?,
        });
    }

    Ok(sections)
}

fn rva_to_offset(sections: &[Section], rva: u32) -> Result<usize> {
    for section in sections {
        let size = section.virtual_size.max(section.raw_size);

        if rva >= section.virtual_address && rva - section.virtual_address < size {
            let offset_in_section = rva - section.virtual_address;

            if offset_in_section >= section.raw_size {
                bail!("address {rva:#x} has no data in the file");
            }

            return Ok((section.raw_offset + offset_in_section) as usize);
        }
    }

    bail!("address {rva:#x} is outside every section")
}


fn coff_header_offset(bytes: &[u8]) -> Result<usize> {
    if bytes.get(0..2) != Some(b"MZ") {
        bail!("not a Windows program file");
    }

    let pe_offset = read_u32(bytes, 0x3c)? as usize;

    if bytes.get(pe_offset..pe_offset + 4) != Some(b"PE\0\0") {
        bail!("not a Windows program file");
    }

    Ok(pe_offset + 4)
}

pub fn read_machine(bytes: &[u8]) -> Result<u16> {
    read_u16(bytes, coff_header_offset(bytes)?)
}

pub fn read_exports(bytes: &[u8]) -> Result<Vec<PeExport>> {
    let coff_header = coff_header_offset(bytes)?;
    let section_count = read_u16(bytes, coff_header + 2)?;
    let optional_header_size = read_u16(bytes, coff_header + 16)? as usize;
    let optional_header = coff_header + 20;

    let data_directories = match read_u16(bytes, optional_header)? {
        PE32_PLUS_MAGIC => optional_header + 112,
        PE32_MAGIC => optional_header + 96,
        other => bail!("unknown optional header {other:#x}"),
    };

    let sections = read_sections(bytes, optional_header + optional_header_size, section_count)?;
    let export_rva = read_u32(bytes, data_directories)?;
    let export_size = read_u32(bytes, data_directories + 4)?;

    if export_rva == 0 {
        return Ok(Vec::new());
    }

    let directory = rva_to_offset(&sections, export_rva)?;
    let name_count = read_u32(bytes, directory + 24)?;
    let function_count = read_u32(bytes, directory + 20)?;

    if name_count > MAX_EXPORTS || function_count > MAX_EXPORTS {
        bail!("export table is implausibly large");
    }

    let functions = rva_to_offset(&sections, read_u32(bytes, directory + 28)?)?;
    let names = rva_to_offset(&sections, read_u32(bytes, directory + 32)?)?;
    let ordinals = rva_to_offset(&sections, read_u32(bytes, directory + 36)?)?;
    let mut exports = Vec::with_capacity(name_count as usize);

    for index in 0..name_count as usize {
        let name_offset = rva_to_offset(&sections, read_u32(bytes, names + index * 4)?)?;
        let ordinal = read_u16(bytes, ordinals + index * 2)? as u32;

        if ordinal >= function_count {
            bail!("export {index} points past the function table");
        }

        let function_rva = read_u32(bytes, functions + ordinal as usize * 4)?;
        let is_forward = function_rva >= export_rva && function_rva - export_rva < export_size;

        let forward = if is_forward {
            Some(read_c_string(bytes, rva_to_offset(&sections, function_rva)?)?)
        } else {
            None
        };

        exports.push(PeExport { name: read_c_string(bytes, name_offset)?, forward });
    }

    Ok(exports)
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_files_that_are_not_programs() {
        assert!(read_exports(b"").is_err());
        assert!(read_exports(b"MZ").is_err());
        assert!(read_exports(&[0u8; 512]).is_err());
    }

    #[test]
    fn rejects_truncated_programs() {
        let mut bytes = vec![0u8; 0x80];
        bytes[0..2].copy_from_slice(b"MZ");
        bytes[0x3c] = 0x40;
        bytes[0x40..0x44].copy_from_slice(b"PE\0\0");

        assert!(read_exports(&bytes).is_err());
    }

    #[cfg(windows)]
    #[test]
    fn reads_names_and_forwards_of_a_system_dll() {
        let system_root = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into());
        let bytes = std::fs::read(std::path::Path::new(&system_root).join("System32").join("kernel32.dll")).unwrap();
        let exports = read_exports(&bytes).unwrap();

        assert!(exports.iter().any(|export| export.name == "CreateFileW"));
        assert!(exports.iter().any(|export| export.forward.as_deref().is_some_and(|target| target.contains('.'))));
    }
}
