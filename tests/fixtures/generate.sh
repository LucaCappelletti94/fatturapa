#!/usr/bin/env bash
# Regenerates the test PKI and the signed invoice fixtures.
# The TSL-IT.xml fixture is downloaded separately with
# curl -sfL -o TSL-IT.xml https://eidas.agid.gov.it/TL/TSL-IT.xml
set -euo pipefail
cd "$(dirname "$0")"

work=$(mktemp -d)
trap 'rm -rf "$work" *.srl' EXIT

invoice=../../spec/samples/IT01234567890_FPR01.xml

# Root CA and intermediate, long-lived.
openssl req -x509 -new -newkey rsa:3072 -nodes \
    -keyout ca.key -out ca.pem -days 3650 \
    -subj "/C=IT/O=fatturapa test/CN=fatturapa test root CA" \
    -addext "basicConstraints=critical,CA:TRUE" \
    -addext "keyUsage=critical,keyCertSign,cRLSign" 2>/dev/null

openssl req -new -newkey rsa:3072 -nodes \
    -keyout intermediate.key -out "$work/intermediate.csr" \
    -subj "/C=IT/O=fatturapa test/CN=fatturapa test intermediate CA" 2>/dev/null
# -not_before a few hours back: the CMS signingTime is second-resolution, so a
# certificate minted in the same second could look NotYetValid to a verifier.
past=$(date -u -d '2 hours ago' +%Y%m%d%H%M%SZ)
openssl x509 -req -in "$work/intermediate.csr" \
    -CA ca.pem -CAkey ca.key -CAcreateserial -out intermediate.pem -days 3650 \
    -not_before "$past" -set_serial 1 \
    -extfile <(printf 'basicConstraints=critical,CA:TRUE\nkeyUsage=critical,keyCertSign,cRLSign\n') 2>/dev/null

# Signers.
openssl req -new -newkey rsa:2048 -nodes \
    -keyout rsa-signer.key -out "$work/rsa-signer.csr" \
    -subj "/C=IT/O=fatturapa test/CN=fatturapa test RSA signer" 2>/dev/null
openssl x509 -req -in "$work/rsa-signer.csr" \
    -CA intermediate.pem -CAkey intermediate.key -CAcreateserial \
    -out rsa-signer.pem -days 3650 -not_before "$past" 2>/dev/null

openssl ecparam -name prime256v1 -genkey -noout -out ec-signer.key
openssl req -new -key ec-signer.key -out "$work/ec-signer.csr" \
    -subj "/C=IT/O=fatturapa test/CN=fatturapa test ECDSA signer" 2>/dev/null
openssl x509 -req -in "$work/ec-signer.csr" \
    -CA intermediate.pem -CAkey intermediate.key -CAcreateserial \
    -out ec-signer.pem -days 3650 -not_before "$past" 2>/dev/null

# A signer valid only in 2020, signed now, for the expired-signer test.
openssl req -new -newkey rsa:2048 -nodes \
    -keyout expired-signer.key -out "$work/expired-signer.csr" \
    -subj "/C=IT/O=fatturapa test/CN=fatturapa test expired signer" 2>/dev/null
openssl x509 -req -in "$work/expired-signer.csr" \
    -CA intermediate.pem -CAkey intermediate.key -CAcreateserial \
    -out expired-signer.pem -not_before 20200101000000Z -not_after 20201231235959Z 2>/dev/null

# A P-521 signer, whose curve the verifier does not support.
openssl ecparam -name secp521r1 -genkey -noout -out p521-signer.key
openssl req -new -key p521-signer.key -out "$work/p521-signer.csr" \
    -subj "/C=IT/O=fatturapa test/CN=fatturapa test P-521 signer" 2>/dev/null
openssl x509 -req -in "$work/p521-signer.csr" \
    -CA intermediate.pem -CAkey intermediate.key -CAcreateserial \
    -out p521-signer.pem -days 3650 -not_before "$past" 2>/dev/null

# An unrelated root, for the wrong-anchor test.
openssl req -x509 -new -newkey rsa:3072 -nodes \
    -keyout other-root.key -out other-root.pem -days 3650 \
    -subj "/C=IT/O=fatturapa other/CN=fatturapa other root CA" \
    -addext "basicConstraints=critical,CA:TRUE" \
    -addext "keyUsage=critical,keyCertSign,cRLSign" 2>/dev/null

# Signers whose certificates carry a specific extended key usage.
openssl ecparam -name prime256v1 -genkey -noout -out eku-docsigning.key
openssl req -new -key eku-docsigning.key -out "$work/eku-docsigning.csr" \
    -subj "/C=IT/O=fatturapa test/CN=fatturapa test document signing" 2>/dev/null
openssl x509 -req -in "$work/eku-docsigning.csr" \
    -CA intermediate.pem -CAkey intermediate.key -CAcreateserial \
    -out eku-docsigning.pem -days 3650 -not_before "$past" \
    -extfile <(printf 'extendedKeyUsage=1.3.6.1.5.5.7.3.36\n') 2>/dev/null

openssl ecparam -name prime256v1 -genkey -noout -out eku-serverauth.key
openssl req -new -key eku-serverauth.key -out "$work/eku-serverauth.csr" \
    -subj "/C=IT/O=fatturapa test/CN=fatturapa test server auth" 2>/dev/null
openssl x509 -req -in "$work/eku-serverauth.csr" \
    -CA intermediate.pem -CAkey intermediate.key -CAcreateserial \
    -out eku-serverauth.pem -days 3650 -not_before "$past" \
    -extfile <(printf 'extendedKeyUsage=serverAuth\n') 2>/dev/null

# A signer whose keyUsage permits only key encipherment.
openssl ecparam -name prime256v1 -genkey -noout -out ku-keyenc.key
openssl req -new -key ku-keyenc.key -out "$work/ku-keyenc.csr" \
    -subj "/C=IT/O=fatturapa test/CN=fatturapa test key encipherment" 2>/dev/null
openssl x509 -req -in "$work/ku-keyenc.csr" \
    -CA intermediate.pem -CAkey intermediate.key -CAcreateserial \
    -out ku-keyenc.pem -days 3650 -not_before "$past" \
    -extfile <(printf 'keyUsage=critical,keyEncipherment\n') 2>/dev/null

# A signer whose keyUsage permits only digital signatures.
openssl ecparam -name prime256v1 -genkey -noout -out ku-digsig.key
openssl req -new -key ku-digsig.key -out "$work/ku-digsig.csr" \
    -subj "/C=IT/O=fatturapa test/CN=fatturapa test digital signature" 2>/dev/null
openssl x509 -req -in "$work/ku-digsig.csr" \
    -CA intermediate.pem -CAkey intermediate.key -CAcreateserial \
    -out ku-digsig.pem -days 3650 -not_before "$past" \
    -extfile <(printf 'keyUsage=critical,digitalSignature\n') 2>/dev/null

# A signer whose only extended key usage is the Microsoft document signing one.
openssl ecparam -name prime256v1 -genkey -noout -out msdocsigning.key
openssl req -new -key msdocsigning.key -out "$work/msdocsigning.csr" \
    -subj "/C=IT/O=fatturapa test/CN=fatturapa test MS document signing" 2>/dev/null
openssl x509 -req -in "$work/msdocsigning.csr" \
    -CA intermediate.pem -CAkey intermediate.key -CAcreateserial \
    -out msdocsigning.pem -days 3650 -not_before "$past" \
    -extfile <(printf 'extendedKeyUsage=1.3.6.1.4.1.311.10.3.12\n') 2>/dev/null

# A P-384 signer.
openssl ecparam -name secp384r1 -genkey -noout -out ec384-signer.key
openssl req -new -key ec384-signer.key -out "$work/ec384-signer.csr" \
    -subj "/C=IT/O=fatturapa test/CN=fatturapa test P-384 signer" 2>/dev/null
openssl x509 -req -in "$work/ec384-signer.csr" \
    -CA intermediate.pem -CAkey intermediate.key -CAcreateserial \
    -out ec384-signer.pem -days 3650 -not_before "$past" 2>/dev/null

# A sibling signer of the ECDSA signer, under the same intermediate. The short
# serial sorts the certificate before the ECDSA signer in the certificates set.
openssl ecparam -name prime256v1 -genkey -noout -out sibling.key
openssl req -new -key sibling.key -out "$work/sibling.csr" \
    -subj "/C=IT/O=fatturapa test/CN=fatturapa test sibling signer" 2>/dev/null
openssl x509 -req -in "$work/sibling.csr" \
    -CA intermediate.pem -CAkey intermediate.key -CAcreateserial \
    -out sibling.pem -days 3650 -not_before "$past" -set_serial 2 2>/dev/null

# The ECDSA signer key in two certificates with validity windows around now.
future=$(date -u -d '1 hour' +%Y%m%d%H%M%SZ)
openssl req -new -key ec-signer.key -out "$work/boundary-future.csr" \
    -subj "/C=IT/O=fatturapa test/CN=fatturapa test boundary future" 2>/dev/null
openssl x509 -req -in "$work/boundary-future.csr" \
    -CA intermediate.pem -CAkey intermediate.key -CAcreateserial \
    -out boundary-future.pem -not_before "$future" \
    -not_after "$(date -u -d '30 days' +%Y%m%d%H%M%SZ)" 2>/dev/null
openssl req -new -key ec-signer.key -out "$work/boundary-past.csr" \
    -subj "/C=IT/O=fatturapa test/CN=fatturapa test boundary past" 2>/dev/null
openssl x509 -req -in "$work/boundary-past.csr" \
    -CA intermediate.pem -CAkey intermediate.key -CAcreateserial \
    -out boundary-past.pem -not_before "$(date -u -d '3 hours ago' +%Y%m%d%H%M%SZ)" \
    -not_after "$(date -u -d '1 hour ago' +%Y%m%d%H%M%SZ)" 2>/dev/null

# The ECDSA signer key in a certificate whose serial collides with the intermediate's.
openssl req -new -key ec-signer.key -out "$work/collision.csr" \
    -subj "/C=IT/O=fatturapa test/CN=fatturapa test collision signer" 2>/dev/null
openssl x509 -req -in "$work/collision.csr" \
    -CA intermediate.pem -CAkey intermediate.key -CAcreateserial \
    -out collision.pem -days 3650 -not_before "$past" -set_serial 1 2>/dev/null

# Two intermediates under the ca, the first with a pathLenConstraint of zero.
openssl req -new -newkey rsa:3072 -nodes \
    -keyout int1.key -out "$work/int1.csr" \
    -subj "/C=IT/O=fatturapa test/CN=fatturapa test CA one" 2>/dev/null
openssl x509 -req -in "$work/int1.csr" \
    -CA ca.pem -CAkey ca.key -CAcreateserial -out int1.pem -days 3650 \
    -not_before "$past" \
    -extfile <(printf 'basicConstraints=critical,CA:TRUE,pathlen:0\nkeyUsage=critical,keyCertSign,cRLSign\n') 2>/dev/null
openssl req -new -newkey rsa:3072 -nodes \
    -keyout int2.key -out "$work/int2.csr" \
    -subj "/C=IT/O=fatturapa test/CN=fatturapa test CA two" 2>/dev/null
openssl x509 -req -in "$work/int2.csr" \
    -CA int1.pem -CAkey int1.key -CAcreateserial -out int2.pem -days 3650 \
    -not_before "$past" \
    -extfile <(printf 'basicConstraints=critical,CA:TRUE\nkeyUsage=critical,keyCertSign,cRLSign\n') 2>/dev/null
openssl ecparam -name prime256v1 -genkey -noout -out signer2.key
openssl req -new -key signer2.key -out "$work/signer2.csr" \
    -subj "/C=IT/O=fatturapa test/CN=fatturapa test path length signer" 2>/dev/null
openssl x509 -req -in "$work/signer2.csr" \
    -CA int2.pem -CAkey int2.key -CAcreateserial \
    -out signer2.pem -days 3650 -not_before "$past" 2>/dev/null

# An intermediate without the CA basic constraint, and a signer under it.
openssl req -new -newkey rsa:3072 -nodes \
    -keyout no-ca-intermediate.key -out "$work/no-ca-intermediate.csr" \
    -subj "/C=IT/O=fatturapa test/CN=fatturapa test not CA" 2>/dev/null
openssl x509 -req -in "$work/no-ca-intermediate.csr" \
    -CA ca.pem -CAkey ca.key -CAcreateserial \
    -out no-ca-intermediate.pem -days 3650 -not_before "$past" \
    -extfile <(printf 'keyUsage=critical,keyCertSign,cRLSign\n') 2>/dev/null
openssl ecparam -name prime256v1 -genkey -noout -out no-ca-signer.key
openssl req -new -key no-ca-signer.key -out "$work/no-ca-signer.csr" \
    -subj "/C=IT/O=fatturapa test/CN=fatturapa test not CA signer" 2>/dev/null
openssl x509 -req -in "$work/no-ca-signer.csr" \
    -CA no-ca-intermediate.pem -CAkey no-ca-intermediate.key -CAcreateserial \
    -out no-ca-signer.pem -days 3650 -not_before "$past" 2>/dev/null

# An intermediate and a signer that share the same serial number. The pool
# exclusion must keep the intermediate in the pool, not drop it.
openssl req -new -newkey rsa:3072 -nodes \
    -keyout int7.key -out "$work/int7.csr" \
    -subj "/C=IT/O=fatturapa test/CN=fatturapa test CA seven" 2>/dev/null
openssl x509 -req -in "$work/int7.csr" \
    -CA ca.pem -CAkey ca.key -CAcreateserial -out int7.pem -days 3650 \
    -not_before "$past" -set_serial 7 \
    -extfile <(printf 'basicConstraints=critical,CA:TRUE\nkeyUsage=critical,keyCertSign,cRLSign\n') 2>/dev/null
openssl ecparam -name prime256v1 -genkey -noout -out signer7.key
openssl req -new -key signer7.key -out "$work/signer7.csr" \
    -subj "/C=IT/O=fatturapa test/CN=fatturapa test serial seven signer" 2>/dev/null
openssl x509 -req -in "$work/signer7.csr" \
    -CA int7.pem -CAkey int7.key -CAcreateserial \
    -out signer7.pem -days 3650 -not_before "$past" -set_serial 7 2>/dev/null

# Two intermediates that share a subject, the lower one issued by the upper
# one, and a signer under the lower one. The pool exclusion must not drop the
# lower intermediate, which shares the signer's issuer.
openssl req -new -newkey rsa:3072 -nodes \
    -keyout cross2.key -out "$work/cross2.csr" \
    -subj "/C=IT/O=fatturapa test/CN=fatturapa test cross CA" 2>/dev/null
openssl x509 -req -in "$work/cross2.csr" \
    -CA ca.pem -CAkey ca.key -CAcreateserial -out cross2.pem -days 3650 \
    -not_before "$past" -set_serial 30 \
    -extfile <(printf 'basicConstraints=critical,CA:TRUE\nkeyUsage=critical,keyCertSign,cRLSign\n') 2>/dev/null
openssl req -new -newkey rsa:3072 -nodes \
    -keyout cross1.key -out "$work/cross1.csr" \
    -subj "/C=IT/O=fatturapa test/CN=fatturapa test cross CA" 2>/dev/null
openssl x509 -req -in "$work/cross1.csr" \
    -CA cross2.pem -CAkey cross2.key -CAcreateserial -out cross1.pem -days 3650 \
    -not_before "$past" -set_serial 31 \
    -extfile <(printf 'basicConstraints=critical,CA:TRUE\nkeyUsage=critical,keyCertSign,cRLSign\n') 2>/dev/null
openssl ecparam -name prime256v1 -genkey -noout -out signer3.key
openssl req -new -key signer3.key -out "$work/signer3.csr" \
    -subj "/C=IT/O=fatturapa test/CN=fatturapa test cross signer" 2>/dev/null
openssl x509 -req -in "$work/signer3.csr" \
    -CA cross1.pem -CAkey cross1.key -CAcreateserial \
    -out signer3.pem -days 3650 -not_before "$past" -set_serial 32 2>/dev/null

# A root with the ca subject and a different key: an intermediate signed by
# it matches the ca by name but not by signature.
openssl req -x509 -new -newkey rsa:3072 -nodes \
    -keyout ca-lookalike.key -out ca-lookalike.pem -days 3650 \
    -subj "/C=IT/O=fatturapa test/CN=fatturapa test root CA" \
    -addext "basicConstraints=critical,CA:TRUE" \
    -addext "keyUsage=critical,keyCertSign,cRLSign" 2>/dev/null
openssl req -new -newkey rsa:3072 -nodes \
    -keyout intermediate-forged.key -out "$work/intermediate-forged.csr" \
    -subj "/C=IT/O=fatturapa test/CN=fatturapa test intermediate CA" 2>/dev/null
openssl x509 -req -in "$work/intermediate-forged.csr" \
    -CA ca-lookalike.pem -CAkey ca-lookalike.key -CAcreateserial \
    -out intermediate-forged.pem -days 3650 -not_before "$past" \
    -extfile <(printf 'basicConstraints=critical,CA:TRUE\nkeyUsage=critical,keyCertSign,cRLSign\n') 2>/dev/null
openssl ecparam -name prime256v1 -genkey -noout -out forged-signer.key
openssl req -new -key forged-signer.key -out "$work/forged-signer.csr" \
    -subj "/C=IT/O=fatturapa test/CN=fatturapa test forged signer" 2>/dev/null
openssl x509 -req -in "$work/forged-signer.csr" \
    -CA intermediate-forged.pem -CAkey intermediate-forged.key -CAcreateserial \
    -out forged-signer.pem -days 3650 -not_before "$past" 2>/dev/null

chain() {
    cat intermediate.pem ca.pem
}

# Signed invoices, CAdES basic with the intermediate and the root included.
# openssl cms writes S/MIME MIME by default; -outform DER gives the raw
# CMS structure.
openssl cms -sign -nodetach -binary -md sha256 -outform DER -in "$invoice" \
    -out invoice-rsa.p7m -signer rsa-signer.pem -inkey rsa-signer.key \
    -certfile <(chain) 2>/dev/null
# OpenSSL 3 records rsaEncryption, not sha384WithRSAEncryption, as the RSA signer algorithm.
openssl cms -sign -nodetach -binary -md sha384 -outform DER -in "$invoice" \
    -out invoice-rsa-sha384.p7m -signer rsa-signer.pem -inkey rsa-signer.key \
    -certfile <(chain) 2>/dev/null
openssl cms -sign -nodetach -binary -md sha256 -outform DER -in "$invoice" \
    -out invoice-ec.p7m -signer ec-signer.pem -inkey ec-signer.key \
    -certfile <(chain) 2>/dev/null
openssl cms -sign -nodetach -binary -md sha256 -outform DER -in "$invoice" \
    -out invoice-expired.p7m -signer expired-signer.pem \
    -inkey expired-signer.key -certfile <(chain) 2>/dev/null
openssl cms -sign -nodetach -binary -md sha256 -outform DER -in "$invoice" \
    -out invoice-p521.p7m -signer p521-signer.pem -inkey p521-signer.key \
    -certfile <(chain) 2>/dev/null
openssl cms -sign -binary -md sha256 -outform DER -in "$invoice" \
    -out invoice-detached.p7m -signer rsa-signer.pem -inkey rsa-signer.key \
    -certfile <(chain) 2>/dev/null

# A P-384 signer, with the two hashes the verifier supports for it.
openssl cms -sign -nodetach -binary -md sha384 -outform DER -in "$invoice" \
    -out invoice-ec-p384.p7m -signer ec384-signer.pem -inkey ec384-signer.key \
    -certfile <(chain) 2>/dev/null
openssl cms -sign -nodetach -binary -md sha256 -outform DER -in "$invoice" \
    -out invoice-ec384-sha256.p7m -signer ec384-signer.pem -inkey ec384-signer.key \
    -certfile <(chain) 2>/dev/null
# An RSA signature over SHA-512, recorded by OpenSSL as rsaEncryption.
openssl cms -sign -nodetach -binary -md sha512 -outform DER -in "$invoice" \
    -out invoice-rsa-sha512.p7m -signer rsa-signer.pem -inkey rsa-signer.key \
    -certfile <(chain) 2>/dev/null
# Signers with specific key usage extensions.
openssl cms -sign -nodetach -binary -md sha256 -outform DER -in "$invoice" \
    -out invoice-eku-docsigning.p7m -signer eku-docsigning.pem -inkey eku-docsigning.key \
    -certfile <(chain) 2>/dev/null
openssl cms -sign -nodetach -binary -md sha256 -outform DER -in "$invoice" \
    -out invoice-eku-serverauth.p7m -signer eku-serverauth.pem -inkey eku-serverauth.key \
    -certfile <(chain) 2>/dev/null
openssl cms -sign -nodetach -binary -md sha256 -outform DER -in "$invoice" \
    -out invoice-ku-keyenc.p7m -signer ku-keyenc.pem -inkey ku-keyenc.key \
    -certfile <(chain) 2>/dev/null
openssl cms -sign -nodetach -binary -md sha256 -outform DER -in "$invoice" \
    -out invoice-ku-digsig.p7m -signer ku-digsig.pem -inkey ku-digsig.key \
    -certfile <(chain) 2>/dev/null
openssl cms -sign -nodetach -binary -md sha256 -outform DER -in "$invoice" \
    -out invoice-msdocsigning.p7m -signer msdocsigning.pem -inkey msdocsigning.key \
    -certfile <(chain) 2>/dev/null
openssl cms -sign -nodetach -binary -md sha256 -outform DER -in "$invoice" \
    -out invoice-int7-signer7.p7m -signer signer7.pem -inkey signer7.key \
    -certfile <(cat int7.pem ca.pem) 2>/dev/null
openssl cms -sign -nodetach -binary -md sha256 -outform DER -in "$invoice" \
    -out invoice-cross.p7m -signer signer3.pem -inkey signer3.key \
    -certfile <(cat cross1.pem cross2.pem ca.pem) 2>/dev/null
# CMS files for the chains that must fail.
openssl cms -sign -nodetach -binary -md sha256 -outform DER -in "$invoice" \
    -out invoice-pathlen.p7m -signer signer2.pem -inkey signer2.key \
    -certfile <(cat int2.pem int1.pem ca.pem) 2>/dev/null
openssl cms -sign -nodetach -binary -md sha256 -outform DER -in "$invoice" \
    -out invoice-no-ca.p7m -signer no-ca-signer.pem -inkey no-ca-signer.key \
    -certfile <(cat no-ca-intermediate.pem ca.pem) 2>/dev/null
openssl cms -sign -nodetach -binary -md sha256 -outform DER -in "$invoice" \
    -out invoice-forged.p7m -signer forged-signer.pem -inkey forged-signer.key \
    -certfile <(cat intermediate-forged.pem ca.pem) 2>/dev/null

# Base64 encodings of the signatures, as SDI and mailers deliver them.
base64 -w 0 invoice-rsa.p7m > invoice-rsa.p7m.base64
base64 -w 0 invoice-ec.p7m > invoice-ec.p7m.base64

# Only the granted QC CA anchors, and the two history-only services keep their fields inside ServiceHistory.
ca_b64=$(openssl x509 -in ca.pem -outform DER | base64 -w 0)
cat > tsl-synthetic.xml <<EOF
<?xml version="1.0" encoding="utf-8"?>
<TrustServiceStatusList>
  <TSPService>
    <ServiceInformation>
      <ServiceTypeIdentifier>http://uri.etsi.org/TrstSvc/Svctype/TSA</ServiceTypeIdentifier>
      <ServiceStatus>http://uri.etsi.org/TrstSvc/Svcstatus/granted</ServiceStatus>
      <X509Certificate>$ca_b64</X509Certificate>
    </ServiceInformation>
  </TSPService>
  <TSPService>
    <ServiceInformation>
      <ServiceTypeIdentifier>http://uri.etsi.org/TrstSvc/Svctype/CA/QC</ServiceTypeIdentifier>
      <ServiceStatus>http://uri.etsi.org/TrstSvc/Svcstatus/withdrawn</ServiceStatus>
      <X509Certificate>$ca_b64</X509Certificate>
    </ServiceInformation>
  </TSPService>
  <TSPService>
    <ServiceInformation>
      <ServiceTypeIdentifier>http://uri.etsi.org/TrstSvc/Svctype/CA/QC</ServiceTypeIdentifier>
      <ServiceStatus>http://uri.etsi.org/TrstSvc/Svcstatus/granted</ServiceStatus>
      <X509Certificate>$ca_b64</X509Certificate>
    </ServiceInformation>
  </TSPService>
  <TSPService>
    <ServiceHistory>
      <ServiceTypeIdentifier>http://uri.etsi.org/TrstSvc/Svctype/CA/QC</ServiceTypeIdentifier>
    </ServiceHistory>
    <ServiceInformation>
      <ServiceStatus>http://uri.etsi.org/TrstSvc/Svcstatus/granted</ServiceStatus>
      <X509Certificate>$ca_b64</X509Certificate>
    </ServiceInformation>
  </TSPService>
  <TSPService>
    <ServiceHistory>
      <X509Certificate>$ca_b64</X509Certificate>
    </ServiceHistory>
    <ServiceInformation>
      <ServiceTypeIdentifier>http://uri.etsi.org/TrstSvc/Svctype/CA/QC</ServiceTypeIdentifier>
      <ServiceStatus>http://uri.etsi.org/TrstSvc/Svcstatus/granted</ServiceStatus>
    </ServiceInformation>
  </TSPService>
</TrustServiceStatusList>
EOF
