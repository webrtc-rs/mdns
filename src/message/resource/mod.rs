pub mod a;
pub mod aaaa;
pub mod cname;
pub mod mx;
pub mod ns;
pub mod opt;
pub mod ptr;
pub mod soa;
pub mod srv;
pub mod txt;

use super::name::*;
use super::packer::*;
use super::*;
use crate::errors::*;

//use a::*;
use aaaa::*;
use cname::*;
use mx::*;
use ns::*;
use opt::*;
use ptr::*;
use soa::*;
use srv::*;
use txt::*;

use std::collections::HashMap;
use std::fmt;

use crate::message::resource::a::AResource;
use util::Error;

// EDNS(0) wire constants.

const EDNS0_VERSION: u32 = 0;
const EDNS0_DNSSEC_OK: u32 = 0x00008000;
const EDNS_VERSION_MASK: u32 = 0x00ff0000;
const EDNS0_DNSSEC_OK_MASK: u32 = 0x00ff8000;

// A Resource is a DNS resource record.
pub struct Resource {
    header: ResourceHeader,
    body: Box<dyn ResourceBody>,
}

impl fmt::Display for Resource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "dnsmessage.Resource{{Header: {}, Body: {}}}",
            self.header, self.body
        )
    }
}

impl Resource {
    // pack appends the wire format of the Resource to msg.
    pub fn pack(
        &self,
        mut msg: Vec<u8>,
        compression: &mut Option<HashMap<String, usize>>,
        compression_off: usize,
    ) -> Result<Vec<u8>, Error> {
        let body_msg = self.body.pack(vec![], compression, compression_off)?;
        let mut header = self.header.clone();
        header.typ = self.body.real_type();
        header.length = body_msg.len() as u16;

        msg = self.header.pack(msg, compression, compression_off)?;
        msg.extend_from_slice(&body_msg);

        Ok(msg)
    }

    pub fn unpack(&mut self, msg: &[u8], mut off: usize) -> Result<usize, Error> {
        off = self.header.unpack(msg, off, 0)?;
        off = self.unpack_resource_body(msg, off)?;
        Ok(off)
    }

    fn unpack_resource_body(&mut self, msg: &[u8], mut off: usize) -> Result<usize, Error> {
        self.body = match self.header.typ {
            DNSType::A => {
                let mut rb = AResource::default();
                off = rb.unpack(msg, off, 0)?;
                Box::new(rb)
            }
            DNSType::NS => {
                let mut rb = NSResource::default();
                off = rb.unpack(msg, off, 0)?;
                Box::new(rb)
            }
            DNSType::CNAME => {
                let mut rb = CNAMEResource::default();
                off = rb.unpack(msg, off, 0)?;
                Box::new(rb)
            }
            DNSType::SOA => {
                let mut rb = SOAResource::default();
                off = rb.unpack(msg, off, 0)?;
                Box::new(rb)
            }
            DNSType::PTR => {
                let mut rb = PTRResource::default();
                off = rb.unpack(msg, off, 0)?;
                Box::new(rb)
            }
            DNSType::MX => {
                let mut rb = MXResource::default();
                off = rb.unpack(msg, off, 0)?;
                Box::new(rb)
            }
            DNSType::TXT => {
                let mut rb = TXTResource::default();
                off = rb.unpack(msg, off, self.header.length as usize)?;
                Box::new(rb)
            }
            DNSType::AAAA => {
                let mut rb = AAAAResource::default();
                off = rb.unpack(msg, off, 0)?;
                Box::new(rb)
            }
            DNSType::SRV => {
                let mut rb = SRVResource::default();
                off = rb.unpack(msg, off, 0)?;
                Box::new(rb)
            }
            DNSType::OPT => {
                let mut rb = OPTResource::default();
                off = rb.unpack(msg, off, self.header.length as usize)?;
                Box::new(rb)
            }
            _ => return Err(ERR_NIL_RESOUCE_BODY.to_owned()),
        };

        Ok(off + self.header.length as usize)
    }

    pub(crate) fn skip(msg: &[u8], off: usize) -> Result<usize, Error> {
        let mut new_off = Name::skip(msg, off)?;
        new_off = DNSType::skip(msg, new_off)?;
        new_off = DNSClass::skip(msg, new_off)?;
        new_off = skip_uint32(msg, new_off)?;
        let (length, mut new_off) = unpack_uint16(msg, new_off)?;
        new_off += length as usize;
        if new_off > msg.len() {
            return Err(ERR_RESOURCE_LEN.to_owned());
        }
        Ok(new_off)
    }
}

// A ResourceHeader is the header of a DNS resource record. There are
// many types of DNS resource records, but they all share the same header.
#[derive(Clone)]
pub struct ResourceHeader {
    // Name is the domain name for which this resource record pertains.
    name: Name,

    // Type is the type of DNS resource record.
    //
    // This field will be set automatically during packing.
    typ: DNSType,

    // Class is the class of network to which this DNS resource record
    // pertains.
    class: DNSClass,

    // TTL is the length of time (measured in seconds) which this resource
    // record is valid for (time to live). All Resources in a set should
    // have the same TTL (RFC 2181 Section 5.2).
    ttl: u32,

    // Length is the length of data in the resource record after the header.
    //
    // This field will be set automatically during packing.
    length: u16,
}

impl fmt::Display for ResourceHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "dnsmessage.ResourceHeader{{Name: {}, Type: {}, Class: {}, TTL: {}, Length: {}}}",
            self.name, self.typ, self.class, self.ttl, self.length,
        )
    }
}

impl ResourceHeader {
    // pack appends the wire format of the ResourceHeader to oldMsg.
    //
    // lenOff is the offset in msg where the Length field was packed.
    fn pack(
        &self,
        mut msg: Vec<u8>,
        compression: &mut Option<HashMap<String, usize>>,
        compression_off: usize,
    ) -> Result<Vec<u8>, Error> {
        msg = self.name.pack(msg, compression, compression_off)?;
        msg = self.typ.pack(msg);
        msg = self.class.pack(msg);
        msg = pack_uint32(msg, self.ttl);
        msg = pack_uint16(msg, self.length);
        Ok(msg)
    }

    fn unpack(&mut self, msg: &[u8], off: usize, _length: usize) -> Result<usize, Error> {
        let mut new_off = off;
        new_off = self.name.unpack(msg, new_off)?;
        new_off = self.typ.unpack(msg, new_off)?;
        new_off = self.class.unpack(msg, new_off)?;
        let (ttl, new_off) = unpack_uint32(msg, new_off)?;
        self.ttl = ttl;
        let (l, new_off) = unpack_uint16(msg, new_off)?;
        self.length = l;

        Ok(new_off)
    }

    // set_edns0 configures h for EDNS(0).
    //
    // The provided ext_rcode must be an extedned RCode.
    pub fn set_edns0(
        &mut self,
        udp_payload_len: u16,
        ext_rcode: RCode,
        dnssec_ok: bool,
    ) -> Result<(), Error> {
        self.name = Name {
            data: ".".to_owned(),
        }; // RFC 6891 section 6.1.2
        self.typ = DNSType::OPT;
        self.class = DNSClass::from(udp_payload_len);
        self.ttl = ((ext_rcode as u32) >> 4) << 24;
        if dnssec_ok {
            self.ttl |= EDNS0_DNSSEC_OK;
        }
        Ok(())
    }

    // dnssec_allowed reports whether the DNSSEC OK bit is set.
    pub fn dnssec_allowed(&self) -> bool {
        self.ttl & EDNS0_DNSSEC_OK_MASK == EDNS0_DNSSEC_OK // RFC 6891 section 6.1.3
    }

    // extended_rcode returns an extended RCode.
    //
    // The provided rcode must be the RCode in DNS message header.
    pub fn extended_rcode(&self, rcode: RCode) -> RCode {
        if self.ttl & EDNS_VERSION_MASK == EDNS0_VERSION {
            // RFC 6891 section 6.1.3
            let ttl = ((self.ttl >> 24) << 4) as u8 | rcode as u8;
            return RCode::from(ttl);
        }
        rcode
    }
}

// A ResourceBody is a DNS resource record minus the header.
pub trait ResourceBody: fmt::Display {
    // real_type returns the actual type of the Resource. This is used to
    // fill in the header Type field.
    fn real_type(&self) -> DNSType;

    // pack packs a Resource except for its header.
    fn pack(
        &self,
        msg: Vec<u8>,
        compression: &mut Option<HashMap<String, usize>>,
        compression_off: usize,
    ) -> Result<Vec<u8>, Error>;

    fn unpack(&mut self, msg: &[u8], off: usize, length: usize) -> Result<usize, Error>;
}
