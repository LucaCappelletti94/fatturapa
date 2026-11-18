"""Signs the XAdES fixtures with lxml and cryptography, an implementation independent of the crate's canonicalization.

Run by ../generate.sh after it recreates the test PKI, from any directory.
"""
import base64, copy, hashlib
from datetime import timedelta
from pathlib import Path
from lxml import etree
from cryptography import x509
from cryptography.hazmat.primitives import hashes, serialization
from cryptography.hazmat.primitives.asymmetric import padding, ec
from cryptography.hazmat.primitives.asymmetric.utils import decode_dss_signature

REPO = Path(__file__).resolve().parents[3]
FIX = REPO / "tests/fixtures"
OUT = FIX / "xades"
INVOICE = REPO / "spec/samples/IT01234567890_FPR01.xml"

DS = "http://www.w3.org/2000/09/xmldsig#"
XADES = "http://uri.etsi.org/01903/v1.3.2#"
EC_NS = "http://www.w3.org/2001/10/xml-exc-c14n#"
F2 = "http://www.w3.org/2002/06/xmldsig-filter2"
INC = "http://www.w3.org/TR/2001/REC-xml-c14n-20010315"
INC_C = INC + "#WithComments"
EXC = "http://www.w3.org/2001/10/xml-exc-c14n#"
EXC_C = EXC + "WithComments"
ENV = DS + "enveloped-signature"
XSLT = "http://www.w3.org/TR/1999/REC-xslt-19991116"
DIG = {"sha1": DS + "sha1", "sha256": "http://www.w3.org/2001/04/xmlenc#sha256",
       "sha384": "http://www.w3.org/2001/04/xmldsig-more#sha384", "sha512": "http://www.w3.org/2001/04/xmlenc#sha512"}
MORE = "http://www.w3.org/2001/04/xmldsig-more#"
SIG = {"rsa-sha1": DS + "rsa-sha1", "rsa-sha256": MORE + "rsa-sha256", "rsa-sha384": MORE + "rsa-sha384",
       "rsa-sha512": MORE + "rsa-sha512", "ecdsa-sha256": MORE + "ecdsa-sha256", "ecdsa-sha384": MORE + "ecdsa-sha384"}
HASH = {"sha1": hashes.SHA1(), "sha256": hashes.SHA256(), "sha384": hashes.SHA384(), "sha512": hashes.SHA512()}


def q(ns, local):
    return "{%s}%s" % (ns, local)


def cert(name):
    return x509.load_pem_x509_certificate((FIX / f"{name}.pem").read_bytes())


def key(name):
    return serialization.load_pem_private_key((FIX / f"{name}.key").read_bytes(), None)


# One hour after the start of the intermediate's validity, inside the validity of every signer but the expired one.
TIME = (cert("intermediate").not_valid_before_utc + timedelta(hours=1)).strftime("%Y-%m-%dT%H:%M:%SZ")


def der(c):
    return c.public_bytes(serialization.Encoding.DER)


def b64(data):
    return base64.b64encode(data).decode()


def der_len(n):
    if n < 128:
        return bytes([n])
    b = n.to_bytes((n.bit_length() + 7) // 8, "big")
    return bytes([0x80 | len(b)]) + b


def tlv(tag, body):
    return bytes([tag]) + der_len(len(body)) + body


def issuer_serial_v2(c):
    serial = c.serial_number.to_bytes((c.serial_number.bit_length() + 8) // 8, "big")
    general_names = tlv(0x30, tlv(0xA4, c.issuer.public_bytes()))
    return tlv(0x30, general_names + tlv(0x02, serial))


def c14n(node, alg, prefixes=None):
    """lxml C14N 1.0 of node (element subtree, or whole ElementTree), per algorithm URI."""
    exclusive = alg in (EXC, EXC_C)
    comments = alg in (INC_C, EXC_C)
    kwargs = {"inclusive_ns_prefixes": prefixes} if exclusive and prefixes else {}
    return etree.tostring(node, method="c14n", exclusive=exclusive, with_comments=comments, **kwargs)


def sub(parent, ns, local, text=None, **attrs):
    element = etree.SubElement(parent, q(ns, local), {k: v for k, v in attrs.items()})
    if text is not None:
        element.text = text
    return element


def build(name, *, signer="rsa-signer", method="rsa-sha256", c14n_method=EXC, c14n_prefixes=None,
          signed_info_comment=False, doc_transforms=("filter2",), doc_digest="sha256", doc_reference=True,
          props_reference=True, props_transforms=(), keyinfo_reference=False, keyinfo_certs=("signer",),
          signing_certificate=None, cert_digest="sha256", issuer_serial=True, signing_cert_of=None,
          extra_references=(), document_comment=False, mutate=None, doctype=False,
          foreign_signature=False, serial_offset=0):
    tree = etree.parse(str(INVOICE), etree.XMLParser(remove_blank_text=False))
    root = tree.getroot()
    if foreign_signature:
        etree.SubElement(root, "{urn:other}Signature", nsmap={"o": "urn:other"}).text = "signed content"
    if document_comment:
        root.insert(0, etree.Comment(" a comment the document reference does not digest "))
    signer_cert = cert(signer)
    signer_key = key(signer)
    certs = {"signer": signer_cert, "intermediate": cert("intermediate"), "outsider": cert("ec-signer")}

    signature = etree.SubElement(root, q(DS, "Signature"), nsmap={"ds": DS})
    signature.set("Id", "Signature1")
    signed_info = sub(signature, DS, "SignedInfo")
    if signed_info_comment:
        signed_info.append(etree.Comment(" kept by the WithComments canonicalization "))
    cm = sub(signed_info, DS, "CanonicalizationMethod", Algorithm=c14n_method)
    if c14n_prefixes:
        ns = etree.SubElement(cm, q(EC_NS, "InclusiveNamespaces"), nsmap={"ec": EC_NS})
        ns.set("PrefixList", " ".join(c14n_prefixes))
    sub(signed_info, DS, "SignatureMethod", Algorithm=SIG[method])

    references = []  # (reference element, uri, transforms, digest name)
    if doc_reference:
        reference = sub(signed_info, DS, "Reference", Id="reference-document", URI="")
        references.append((reference, "", doc_transforms, doc_digest))
    if props_reference:
        reference = sub(signed_info, DS, "Reference", Id="reference-signedproperties",
                        Type="http://uri.etsi.org/01903#SignedProperties", URI="#SignedProperties1")
        references.append((reference, "#SignedProperties1", props_transforms, "sha256"))
    if keyinfo_reference:
        reference = sub(signed_info, DS, "Reference", URI="#KeyInfo1")
        references.append((reference, "#KeyInfo1", (), "sha256"))
    for uri, transforms in extra_references:
        reference = sub(signed_info, DS, "Reference", URI=uri)
        references.append((reference, uri, transforms, "sha256"))

    for reference, uri, transforms, digest in references:
        if transforms:
            tr = sub(reference, DS, "Transforms")
            for t in transforms:
                if t == "filter2":
                    element = sub(tr, DS, "Transform", Algorithm=F2)
                    xp = etree.SubElement(element, q(F2, "XPath"), nsmap={None: F2})
                    xp.set("Filter", "subtract")
                    xp.text = "/descendant::ds:Signature"
                elif t == "enveloped":
                    sub(tr, DS, "Transform", Algorithm=ENV)
                else:
                    sub(tr, DS, "Transform", Algorithm=t)
        sub(reference, DS, "DigestMethod", Algorithm=DIG[digest])
        sub(reference, DS, "DigestValue")

    value = sub(signature, DS, "SignatureValue")
    key_info = sub(signature, DS, "KeyInfo", Id="KeyInfo1")
    x509_data = sub(key_info, DS, "X509Data")
    for which in keyinfo_certs:
        sub(x509_data, DS, "X509Certificate", b64(der(certs[which])))
    obj = sub(signature, DS, "Object")
    qp = etree.SubElement(obj, q(XADES, "QualifyingProperties"), nsmap={"xades": XADES})
    qp.set("Target", "#Signature1")
    sp = sub(qp, XADES, "SignedProperties", Id="SignedProperties1")
    ssp = sub(sp, XADES, "SignedSignatureProperties")
    sub(ssp, XADES, "SigningTime", TIME)
    if signing_certificate:
        referenced = certs[signing_cert_of or "signer"]
        sc = sub(ssp, XADES, "SigningCertificate" if signing_certificate == 1 else "SigningCertificateV2")
        c = sub(sc, XADES, "Cert")
        cd = sub(c, XADES, "CertDigest")
        sub(cd, DS, "DigestMethod", Algorithm=DIG[cert_digest])
        sub(cd, DS, "DigestValue", b64(hashlib.new(cert_digest, der(referenced)).digest()))
        if issuer_serial and signing_certificate == 1:
            isr = sub(c, XADES, "IssuerSerial")
            sub(isr, DS, "X509IssuerName", ", ".join(rdn.rfc4514_string() for rdn in reversed(referenced.issuer.rdns)))
            sub(isr, DS, "X509SerialNumber", str(referenced.serial_number + serial_offset))
        elif issuer_serial:
            sub(c, XADES, "IssuerSerialV2", b64(issuer_serial_v2(referenced)))

    for reference, uri, transforms, digest in references:
        algs = [t for t in transforms if t in (INC, INC_C, EXC, EXC_C)]
        alg = algs[0] if algs else INC
        if uri == "":
            stripped = copy.deepcopy(tree)
            for s in stripped.getroot().iter(q(DS, "Signature")):
                s.getparent().remove(s)
            # A same-document URI drops comments before any canonicalization.
            data = c14n(stripped, INC if alg == INC_C else EXC if alg == EXC_C else alg)
        elif uri.startswith("#"):
            target = root.xpath("//*[@Id=$i]", i=uri[1:])[0]
            data = c14n(target, INC if alg == INC_C else EXC if alg == EXC_C else alg)
        else:
            data = b"external"
        reference.find(q(DS, "DigestValue")).text = b64(hashlib.new(digest, data).digest())

    signed = c14n(signed_info, c14n_method, c14n_prefixes)
    hash_name = method.split("-")[1]
    if method.startswith("rsa"):
        raw = signer_key.sign(signed, padding.PKCS1v15(), HASH[hash_name])
    else:
        r, s = decode_dss_signature(signer_key.sign(signed, ec.ECDSA(HASH[hash_name])))
        size = (signer_key.curve.key_size + 7) // 8
        raw = r.to_bytes(size, "big") + s.to_bytes(size, "big")
    value.text = b64(raw)

    if mutate:
        mutate(tree)
    data = etree.tostring(tree, xml_declaration=True, encoding="UTF-8")
    if doctype:
        head, rest = data.split(b"?>", 1)
        data = head + b"?>\n<!DOCTYPE p:FatturaElettronica>" + rest
    (OUT / f"{name}.xml").write_bytes(data)


def find(tree, xpath):
    return tree.getroot().xpath(xpath, namespaces={"ds": DS, "xades": XADES, "p": "http://ivaservizi.agenziaentrate.gov.it/docs/xsd/fatture/v1.2"})[0]


def change_invoice(tree):
    element = find(tree, "//Descrizione")
    element.text = element.text + " changed"


def change_signing_time(tree):
    find(tree, "//xades:SigningTime").text = "2026-11-17T11:00:00Z"


def change_reference_id(tree):
    find(tree, "//ds:Reference[@Id='reference-document']").set("Id", "reference-changed")


def duplicate_id(tree):
    obj = etree.SubElement(find(tree, "//ds:Signature"), q(DS, "Object"))
    etree.SubElement(obj, "Decoy", Id="SignedProperties1")


# The SDI message shape: filter2, no explicit C14N, a signed KeyInfo, SigningTime only.
build("sdi-rsa", keyinfo_reference=True)
# The AgID list shape: enveloped + exclusive C14N, SigningCertificateV2 with IssuerSerialV2.
TSL = dict(method="rsa-sha512", doc_transforms=("enveloped", EXC), doc_digest="sha512",
           props_transforms=(EXC,), keyinfo_certs=("signer", "intermediate"), signing_certificate=2,
           cert_digest="sha512")
build("tsl-rsa-sha512", **TSL)
# Inclusive C14N, ECDSA P-256, SigningCertificate v1 with an IssuerSerial name spelled with spaces.
build("inclusive-ec", signer="ec-signer", method="ecdsa-sha256", c14n_method=INC, doc_transforms=("enveloped", INC),
      doc_digest="sha384", keyinfo_certs=("signer", "intermediate"), signing_certificate=1)
# Comments: the document comment is not digested, the SignedInfo comment is signed, PrefixList renders p.
build("comments-ec384", signer="ec384-signer", method="ecdsa-sha384", c14n_method=EXC_C, c14n_prefixes=["p"],
      signed_info_comment=True, doc_transforms=("enveloped", EXC_C), document_comment=True,
      keyinfo_certs=("signer", "intermediate"), signing_certificate=2, issuer_serial=False)
# RSA SHA-384 over inclusive C14N with comments and filter2 followed by inclusive C14N.
build("inclusive-rsa-sha384", method="rsa-sha384", c14n_method=INC_C, signed_info_comment=True,
      doc_transforms=("filter2", INC), keyinfo_certs=("signer", "intermediate"), signing_certificate=2)

# Negative cases, each one change from a verifying shape.
build("invoice-changed", mutate=change_invoice, **TSL)
build("properties-changed", mutate=change_signing_time, **TSL)
build("signed-info-changed", mutate=change_reference_id, **TSL)
build("no-document-reference", doc_reference=False, **TSL)
build("no-properties-reference", props_reference=False, **TSL)
build("unprotected-signer")
build("other-certificate", signing_cert_of="outsider", **TSL)
build("ambiguous-id", mutate=duplicate_id, **TSL)
build("external-reference", extra_references=[("http://example.com/invoice.xml", ())], **TSL)
build("xslt-transform", **{**TSL, "doc_transforms": ("enveloped", XSLT)})
build("rsa-sha1", **{**TSL, "method": "rsa-sha1"})
build("sha1-digest", **{**TSL, "doc_digest": "sha1"})
build("doctype", doctype=True, **TSL)
build("expired-signer", signer="expired-signer", **{k: v for k, v in TSL.items() if k != "signer"})
# A filter2 document reference keeps an element named Signature outside the XMLDSig namespace.
build("foreign-signature-element", foreign_signature=True, keyinfo_reference=True)
build("issuer-serial-mismatch", signer="ec-signer", method="ecdsa-sha256", c14n_method=INC,
      doc_transforms=("enveloped", INC), keyinfo_certs=("signer", "intermediate"), signing_certificate=1,
      serial_offset=1)
