use crate::dvd::*;
use zerocopy::FromBytes;

pub struct Iso {
    pub header: Header,
    pub header_info: HeaderInfo,
    pub apploader: Apploader,
    pub filesystem: FstNode,
    data: Vec<u8>,
}

impl Iso {
    pub fn parse(data: Vec<u8>) -> Self {
        let header = Header::read_from_bytes(&data[DVD_HEADER_OFFSET..DVD_HEADER_OFFSET + DVD_HEADER_SIZE]).unwrap();
        let header_info =
            HeaderInfo::read_from_bytes(&data[DVD_HEADER_INFO_OFFSET..DVD_HEADER_INFO_OFFSET + DVD_HEADER_INFO_SIZE])
                .unwrap();
        let apploader =
            Apploader::read_from_bytes(&data[DVD_APPLOADER_OFFSET..DVD_APPLOADER_OFFSET + DVD_APPLOADER_SIZE]).unwrap();

        let fst_start = header.offset_filesystem.get() as usize;
        let fst_end = fst_start + header.filesystem_size.get() as usize;
        let filesystem = FstNode::parse(&data[fst_start..fst_end]);

        Iso {
            header,
            header_info,
            apploader,
            filesystem,
            data,
        }
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }
}

impl crate::Dvd for Iso {
    fn header(&self) -> &Header {
        &self.header
    }

    fn apploader(&self) -> &Apploader {
        &self.apploader
    }

    fn read_disc_into(&self, offset: usize, buf: &mut [u8]) {
        buf.copy_from_slice(&self.data[offset..offset + buf.len()]);
    }
}
