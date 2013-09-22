#[link(name = "retf",
       vers = "0.1",
       uuid = "7ab267cc-ec76-4af2-84e6-ee4c3696d000")];
#[crate_type = "lib"];

use std::io;
use std::str::from_chars;

enum ErlangTerm {
    Integer(i32),
    Float(f64),
    Atom(@str),
    Reference(@ErlangTerm, @[u32], u8),
    Port(@ErlangTerm, u32, u8),
    Pid(@ErlangTerm, u32, u32, u8),
    Tuple(@[@ErlangTerm]),
    String(@str),
    List(@[@ErlangTerm]),
    Binary(@[u8]),
    BigInteger(u32, u8, @[u8]),
    Fun(u32, u8, @[u8,..16], u32, u32, 
        @ErlangTerm, @ErlangTerm, @ErlangTerm, @ErlangTerm, @ErlangTerm),
    Export(@ErlangTerm, @ErlangTerm, @ErlangTerm),
    BitBinary(u8, @[u8]),
    Nil
}

pub struct Decoder {
    reader: @io::Reader
}

impl Decoder {
    fn new(reader: @io::Reader) -> Decoder {
        Decoder {
            reader : reader
        }
    }

    priv fn parse_small_integer(&self) -> @ErlangTerm {
        let v = self.reader.read_u8();
        @Integer(v as i32)
    }

    priv fn parse_integer(&self) -> @ErlangTerm {
        let v = self.reader.read_be_i32();
        @Integer(v)
    }

    priv fn parse_atom(&self) -> @ErlangTerm {
        let l = self.reader.read_be_u16();
        let v = self.reader.read_chars(l as uint);
        let s = from_chars(v);
        @Atom(s.to_managed())
    }

    fn parse(&self) -> @ErlangTerm {
        let tag = self.reader.read_u8();
        match tag {
            97 => { self.parse_small_integer() },
            98 => { self.parse_integer() },
            //99 => { parse float},
            100 => { self.parse_atom() },
            _  => fail!("Unknown etf tag")
        }        
    }
    
}

// ---------------------
// Some testz goez here!

#[cfg(test)]
fn test_parse(bytes: &[u8], f: &fn(@ErlangTerm)) {
    io::with_bytes_reader(bytes, |rd| {
        let decoder = Decoder::new(rd);
        f(decoder.parse())
    });
}

#[cfg(test)]
#[test]
fn deserialize_int_test() {
    test_parse([97, 1], |i| {
        match i {
            @Integer(1) => (),
            _ => fail!()
        }});
    test_parse([98, 119, 53, 148, 0], |i| {
        match i {
            @Integer(2000000000) => (),
            _ => fail!()
        }});
}

#[cfg(test)]
#[test]
fn deserialize_atom_test() {
    test_parse([100, 0, 2, 111, 107], |i| {
        match i {
            @Atom(string) => assert_eq!(string, @"ok"),
            _ => fail!()
        }});
    /*test_parse([98, 119, 53, 148, 0], |i| {
        match i {
            @Integer(2000000000) => (),
            _ => fail!()
        }});*/
}