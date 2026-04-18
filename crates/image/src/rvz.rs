use crate::dvd::{Apploader, DVD_APPLOADER_OFFSET, DVD_APPLOADER_SIZE, DVD_HEADER_OFFSET, DVD_HEADER_SIZE, Header};
use zerocopy::FromBytes;

const RVZ_MAGIC: [u8; 4] = [b'R', b'V', b'Z', 0x01];

const H1_SIZE: usize = 0x48;
const H1_ISO_FILE_SIZE: usize = 0x24;

const H2_DISC_TYPE: usize = 0x00;
const H2_COMPRESSION_TYPE: usize = 0x04;
const H2_CHUNK_SIZE: usize = 0x0C;
const H2_DISC_HEADER: usize = 0x10;
const H2_NUM_PARTITIONS: usize = 0x90;
const H2_NUM_RAW_DATA: usize = 0xB4;
const H2_RAW_DATA_OFFSET: usize = 0xB8;
const H2_RAW_DATA_SIZE: usize = 0xC0;
const H2_NUM_GROUPS: usize = 0xC4;
const H2_GROUPS_OFFSET: usize = 0xC8;
const H2_GROUPS_SIZE: usize = 0xD0;

const RAW_DATA_ENTRY_SIZE: usize = 0x18;
const RVZ_GROUP_ENTRY_SIZE: usize = 0x0C;

const SECTOR_SIZE: u64 = 0x8000;
const RVZ_JUNK_BLOCK_SIZE: usize = 0x8000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Compression {
    None,
    Zstd,
}

#[derive(Clone)]
struct RawData {
    data_offset: u64,
    data_size: u64,
    group_index: u32,
    num_groups: u32,
}

#[derive(Clone, Copy)]
struct GroupEntry {
    file_offset: u64,
    stored_size: u32,
    is_compressed: bool,
    rvz_packed_size: u32,
}

pub struct Rvz {
    header: Header,
    apploader: Apploader,
    disc_header: [u8; 0x80],
    iso_size: u64,
    chunk_size: u32,
    compression: Compression,
    raw_data: Vec<RawData>,
    groups: Vec<GroupEntry>,
    file_data: Vec<u8>,
}

impl Rvz {
    pub fn parse(data: Vec<u8>) -> Self {
        assert!(data.len() >= H1_SIZE, "RVZ file too small for header");
        assert_eq!(&data[0..4], &RVZ_MAGIC, "not an RVZ file (bad magic)");

        let iso_size = self::be64(&data, H1_ISO_FILE_SIZE);
        let h2 = H1_SIZE;

        let disc_type = self::be32(&data, h2 + H2_DISC_TYPE);
        assert_eq!(disc_type, 1, "only GameCube RVZ is supported (disc_type=1)");

        let num_partitions = self::be32(&data, h2 + H2_NUM_PARTITIONS);
        assert_eq!(num_partitions, 0, "Wii partitions are not supported");

        let compression = match self::be32(&data, h2 + H2_COMPRESSION_TYPE) {
            0 => Compression::None,
            5 => Compression::Zstd,
            other => panic!("unsupported RVZ compression type {other} (only None/Zstd supported)"),
        };
        let chunk_size = self::be32(&data, h2 + H2_CHUNK_SIZE);

        let mut disc_header = [0u8; 0x80];
        disc_header.copy_from_slice(&data[h2 + H2_DISC_HEADER..h2 + H2_DISC_HEADER + 0x80]);

        let num_raw_data = self::be32(&data, h2 + H2_NUM_RAW_DATA);
        let raw_data_off = self::be64(&data, h2 + H2_RAW_DATA_OFFSET);
        let raw_data_compressed_size = self::be32(&data, h2 + H2_RAW_DATA_SIZE);
        let num_groups = self::be32(&data, h2 + H2_NUM_GROUPS);
        let groups_off = self::be64(&data, h2 + H2_GROUPS_OFFSET);
        let groups_compressed_size = self::be32(&data, h2 + H2_GROUPS_SIZE);

        let raw_data_blob = self::decompress_metadata(
            &data[raw_data_off as usize..raw_data_off as usize + raw_data_compressed_size as usize],
            compression,
            (num_raw_data as usize) * RAW_DATA_ENTRY_SIZE,
        );
        let raw_data = self::parse_raw_data(&raw_data_blob, num_raw_data);

        let groups_blob = self::decompress_metadata(
            &data[groups_off as usize..groups_off as usize + groups_compressed_size as usize],
            compression,
            (num_groups as usize) * RVZ_GROUP_ENTRY_SIZE,
        );
        let groups = self::parse_groups(&groups_blob, num_groups);

        let mut header_bytes = [0u8; DVD_HEADER_SIZE];
        self::read_into(
            &data,
            &disc_header,
            chunk_size,
            compression,
            &raw_data,
            &groups,
            DVD_HEADER_OFFSET as u64,
            &mut header_bytes,
        );
        let header = Header::read_from_bytes(&header_bytes).expect("invalid DVD header");

        let mut apploader_bytes = [0u8; DVD_APPLOADER_SIZE];
        self::read_into(
            &data,
            &disc_header,
            chunk_size,
            compression,
            &raw_data,
            &groups,
            DVD_APPLOADER_OFFSET as u64,
            &mut apploader_bytes,
        );
        let apploader = Apploader::read_from_bytes(&apploader_bytes).expect("invalid apploader");

        Rvz {
            header,
            apploader,
            disc_header,
            iso_size,
            chunk_size,
            compression,
            raw_data,
            groups,
            file_data: data,
        }
    }
}

fn parse_raw_data(blob: &[u8], count: u32) -> Vec<RawData> {
    let mut out = Vec::with_capacity(count as usize);

    for i in 0..count as usize {
        let base = i * RAW_DATA_ENTRY_SIZE;
        let nominal_offset = self::be64(blob, base);
        let nominal_size = self::be64(blob, base + 0x08);
        let group_index = self::be32(blob, base + 0x10);
        let num_groups = self::be32(blob, base + 0x14);

        let skipped = nominal_offset % SECTOR_SIZE;
        out.push(RawData {
            data_offset: nominal_offset - skipped,
            data_size: nominal_size + skipped,
            group_index,
            num_groups,
        });
    }

    out
}

fn parse_groups(blob: &[u8], count: u32) -> Vec<GroupEntry> {
    let mut out = Vec::with_capacity(count as usize);

    for i in 0..count as usize {
        let base = i * RVZ_GROUP_ENTRY_SIZE;
        let data_off4 = self::be32(blob, base);
        let data_size_raw = self::be32(blob, base + 0x04);
        let rvz_packed_size = self::be32(blob, base + 0x08);

        out.push(GroupEntry {
            file_offset: (data_off4 as u64) << 2,
            stored_size: data_size_raw & 0x7FFF_FFFF,
            is_compressed: (data_size_raw & 0x8000_0000) != 0,
            rvz_packed_size,
        });
    }

    out
}

fn decompress_metadata(payload: &[u8], compression: Compression, expected: usize) -> Vec<u8> {
    let decompressed = match compression {
        Compression::None => payload.to_vec(),
        Compression::Zstd => zstd::decode_all(std::io::Cursor::new(payload)).expect("failed to zstd-decode metadata"),
    };

    assert_eq!(
        decompressed.len(),
        expected,
        "RVZ metadata size mismatch (got {}, want {})",
        decompressed.len(),
        expected
    );

    decompressed
}

fn read_into(
    file: &[u8],
    disc_header: &[u8; 0x80],
    chunk_size: u32,
    compression: Compression,
    raw_data: &[RawData],
    groups: &[GroupEntry],
    mut offset: u64,
    buf: &mut [u8],
) {
    let mut buf_off = 0usize;
    let mut remaining = buf.len();

    if offset < 0x80 {
        let take = core::cmp::min(0x80 - offset as usize, remaining);
        buf[buf_off..buf_off + take].copy_from_slice(&disc_header[offset as usize..offset as usize + take]);
        buf_off += take;
        offset += take as u64;
        remaining -= take;
    }

    while remaining > 0 {
        let rd = raw_data
            .iter()
            .find(|r| offset >= r.data_offset && offset < r.data_offset + r.data_size)
            .expect("disc offset not covered by any raw-data entry");

        let within = offset - rd.data_offset;
        let group_idx = (within / chunk_size as u64) as u32;
        assert!(group_idx < rd.num_groups, "group index out of range");

        let group_offset_in_data = (group_idx as u64) * (chunk_size as u64);
        let this_chunk_size = core::cmp::min(chunk_size as u64, rd.data_size - group_offset_in_data) as usize;

        let group = &groups[(rd.group_index + group_idx) as usize];
        let chunk = self::decompress_group(file, group, this_chunk_size, compression, group_offset_in_data);

        let byte_off_in_chunk = (within - group_offset_in_data) as usize;
        let avail = this_chunk_size - byte_off_in_chunk;
        let take = core::cmp::min(avail, remaining);
        buf[buf_off..buf_off + take].copy_from_slice(&chunk[byte_off_in_chunk..byte_off_in_chunk + take]);

        buf_off += take;
        offset += take as u64;
        remaining -= take;
    }
}

fn decompress_group(
    file: &[u8],
    group: &GroupEntry,
    this_chunk_size: usize,
    file_compression: Compression,
    group_offset_in_data: u64,
) -> Vec<u8> {
    if group.stored_size == 0 {
        return vec![0u8; this_chunk_size];
    }

    let start = group.file_offset as usize;
    let end = start + group.stored_size as usize;
    let payload = &file[start..end];

    let effective = if group.is_compressed {
        file_compression
    } else {
        Compression::None
    };

    let decompressed: Vec<u8> = match effective {
        Compression::None => payload.to_vec(),
        Compression::Zstd => zstd::decode_all(std::io::Cursor::new(payload)).expect("failed to zstd-decode group"),
    };

    if group.rvz_packed_size == 0 {
        let mut out = vec![0u8; this_chunk_size];
        let n = core::cmp::min(decompressed.len(), this_chunk_size);
        out[..n].copy_from_slice(&decompressed[..n]);
        out
    } else {
        assert_eq!(
            decompressed.len(),
            group.rvz_packed_size as usize,
            "rvz_packed_size mismatch"
        );

        self::rvz_unpack(&decompressed, this_chunk_size, group_offset_in_data)
    }
}

fn rvz_unpack(src: &[u8], out_len: usize, data_offset: u64) -> Vec<u8> {
    let mut out = vec![0u8; out_len];
    let mut cursor = 0usize;
    let mut out_pos = 0usize;

    while out_pos < out_len {
        let hdr = u32::from_be_bytes(src[cursor..cursor + 4].try_into().unwrap());
        cursor += 4;

        let is_junk = (hdr & 0x8000_0000) != 0;
        let size = (hdr & 0x7FFF_FFFF) as usize;

        if is_junk {
            let mut seed = [0u8; 68];
            seed.copy_from_slice(&src[cursor..cursor + 68]);
            cursor += 68;

            let mut lfg = Lfg::new(&seed);
            lfg.forward_n((data_offset as usize) % RVZ_JUNK_BLOCK_SIZE);
            lfg.get_bytes(&mut out[out_pos..out_pos + size]);
        } else {
            out[out_pos..out_pos + size].copy_from_slice(&src[cursor..cursor + size]);
            cursor += size;
        }

        out_pos += size;
    }

    out
}

const LFG_K: usize = 521;
const LFG_J: usize = 32;
const LFG_STATE_BYTES: usize = LFG_K * 4;

struct Lfg {
    buffer: [u32; LFG_K],
    position_bytes: usize,
}

impl Lfg {
    fn new(seed: &[u8; 68]) -> Self {
        let mut buffer = [0u32; LFG_K];

        for i in 0..17 {
            buffer[i] = u32::from_be_bytes(seed[i * 4..i * 4 + 4].try_into().unwrap());
        }

        let mut lfg = Self {
            buffer,
            position_bytes: 0,
        };

        lfg.initialize();

        lfg
    }

    fn initialize(&mut self) {
        for i in 17..LFG_K {
            self.buffer[i] = (self.buffer[i - 17] << 23) ^ (self.buffer[i - 16] >> 9) ^ self.buffer[i - 1];
        }

        for x in self.buffer.iter_mut() {
            *x = (*x & 0xFF00_FFFF) | ((*x >> 2) & 0x00FF_0000);
        }

        for _ in 0..4 {
            self.forward();
        }
    }

    #[inline(always)]
    fn forward(&mut self) {
        for i in 0..LFG_J {
            self.buffer[i] ^= self.buffer[i + LFG_K - LFG_J];
        }

        for i in LFG_J..LFG_K {
            self.buffer[i] ^= self.buffer[i - LFG_J];
        }
    }

    fn forward_n(&mut self, count: usize) {
        self.position_bytes += count;

        while self.position_bytes >= LFG_STATE_BYTES {
            self.forward();
            self.position_bytes -= LFG_STATE_BYTES;
        }
    }

    #[inline(always)]
    fn get_byte(&mut self) -> u8 {
        let word_idx = self.position_bytes / 4;
        let byte_in_word = self.position_bytes % 4;
        let byte = (self.buffer[word_idx] >> ((3 - byte_in_word) * 8)) as u8;

        self.position_bytes += 1;

        if self.position_bytes == LFG_STATE_BYTES {
            self.forward();
            self.position_bytes = 0;
        }

        byte
    }

    fn get_bytes(&mut self, out: &mut [u8]) {
        for b in out.iter_mut() {
            *b = self.get_byte();
        }
    }
}

impl crate::Dvd for Rvz {
    fn header(&self) -> &Header {
        &self.header
    }

    fn apploader(&self) -> &Apploader {
        &self.apploader
    }

    fn read_disc_into(&self, offset: usize, buf: &mut [u8]) {
        assert!(
            (offset as u64) + (buf.len() as u64) <= self.iso_size,
            "read past end of disc image"
        );

        self::read_into(
            &self.file_data,
            &self.disc_header,
            self.chunk_size,
            self.compression,
            &self.raw_data,
            &self.groups,
            offset as u64,
            buf,
        );
    }
}

#[inline(always)]
fn be32(data: &[u8], off: usize) -> u32 {
    u32::from_be_bytes(data[off..off + 4].try_into().unwrap())
}

#[inline(always)]
fn be64(data: &[u8], off: usize) -> u64 {
    u64::from_be_bytes(data[off..off + 8].try_into().unwrap())
}
