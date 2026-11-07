//! Byte-level reading of every `SDI` message type, for panics.

#![no_main]

use fatturapa::Document;
use fatturapa::{
    AttestazioneTrasmissioneFattura, MetadatiInvioFile, NotificaDecorrenzaTermini,
    NotificaEsito, NotificaEsitoCommittente, NotificaMancataConsegna, NotificaScarto,
    RicevutaConsegna, ScartoEsitoCommittente,
};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|input: &[u8]| {
    let _ = AttestazioneTrasmissioneFattura::from_xml_slice(input);
    let _ = MetadatiInvioFile::from_xml_slice(input);
    let _ = NotificaDecorrenzaTermini::from_xml_slice(input);
    let _ = NotificaEsitoCommittente::from_xml_slice(input);
    let _ = NotificaEsito::from_xml_slice(input);
    let _ = NotificaMancataConsegna::from_xml_slice(input);
    let _ = NotificaScarto::from_xml_slice(input);
    let _ = RicevutaConsegna::from_xml_slice(input);
    let _ = ScartoEsitoCommittente::from_xml_slice(input);
});
