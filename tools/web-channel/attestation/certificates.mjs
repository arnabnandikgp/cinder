// Candidate verifier plus deliberately closed AWS-shaped certificate policy.
// PKI.js alone is NOT equivalent to OpenSSL X509_STRICT (notably pathLen).
import * as asn1js from 'asn1js';
import { Certificate, CertificateChainValidationEngine, CryptoEngine, BasicConstraints,
  AuthorityKeyIdentifier, CRLDistributionPoints } from 'pkijs';
import { bytes, equal } from './encoding.mjs';
const BC = '2.5.29.19', KU = '2.5.29.15', SKI = '2.5.29.14', AKI = '2.5.29.35';
const CRLDP = '2.5.29.31';
const EC = '1.2.840.10045.2.1', P384 = '1.3.132.0.34', ES384 = '1.2.840.10045.4.3.3';
const supported = new Set([BC, KU, SKI, AKI, CRLDP]);

function positiveInteger(b, cap) {
  return b.length > 0 && b.length <= cap && !(b[0] & 128) && b.some(v => v !== 0)
    && !(b.length > 1 && b[0] === 0 && b[1] < 128);
}
// The fixed AWS path can use a full 160-bit positive serial. Its minimal DER
// INTEGER then needs a leading sign octet. Bound the magnitude, not that pad;
// this does not allow a 21-byte magnitude, negative/zero or nonminimal values.
export function certificateSerial(b) {
  return b instanceof Uint8Array && positiveInteger(b,21)
    && b.length - Number(b[0] === 0) <= 20;
}
function signatureShape(cert) {
  const value = cert.signatureValue.valueBlock, b = value.valueHexView;
  if (value.unusedBits !== 0 || b.length > 104 || b[0] !== 0x30 || b[1] >= 128 || b[1] + 2 !== b.length) throw Error('certificate signature DER');
  let at = 2;
  for (let n = 0; n < 2; n++) {
    if (b[at++] !== 2) throw Error('certificate signature integer');
    const length = b[at++];
    if (at + length > b.length || !positiveInteger(b.subarray(at,at + length),49)) throw Error('certificate signature integer');
    at += length;
  }
  if (at !== b.length) throw Error('certificate signature trailing');
}

function certificate(der, at, ca) {
  bytes(der);
  if (!der.length || der.length > 1024) throw Error('certificate bound');
  const decoded = asn1js.fromBER(der);
  if (decoded.offset !== der.length || decoded.result.error) throw Error('certificate ASN.1');
  const cert = new Certificate({ schema: decoded.result });
  // Canonical re-encoding must reproduce the signed object, not a prefix or BER
  // variant. This is a shape restriction, not a general-purpose DER validator.
  if (!equal(new Uint8Array(cert.toSchema(true).toBER(false)), der) || cert.version !== 2) throw Error('certificate encoding/version');
  const serial = cert.serialNumber.valueBlock.valueHexView;
  if (!certificateSerial(serial)) throw Error('serial');
  signatureShape(cert);
  for (const algorithm of [cert.signature, cert.signatureAlgorithm]) {
    if (algorithm.algorithmId !== ES384 || 'algorithmParams' in algorithm) throw Error('certificate signature algorithm');
  }
  const pub = cert.subjectPublicKeyInfo;
  if (pub.algorithm.algorithmId !== EC || !(pub.algorithm.algorithmParams instanceof asn1js.ObjectIdentifier)
      || pub.algorithm.algorithmParams.valueBlock.toString() !== P384
      || pub.subjectPublicKey.valueBlock.unusedBits !== 0
      || pub.subjectPublicKey.valueBlock.valueHexView.length !== 97
      || pub.subjectPublicKey.valueBlock.valueHexView[0] !== 4) throw Error('P384 key');
  if (!(cert.notBefore.value.getTime() <= at && at < cert.notAfter.value.getTime())) throw Error('certificate time');
  const extensions = new Map();
  for (const ext of cert.extensions ?? []) {
    if (extensions.has(ext.extnID) || !supported.has(ext.extnID) || !ext.parsedValue) throw Error('unsupported/duplicate extension');
    const type = ({[BC]:BasicConstraints,[KU]:asn1js.BitString,[SKI]:asn1js.OctetString,
      [AKI]:AuthorityKeyIdentifier,[CRLDP]:CRLDistributionPoints})[ext.extnID];
    if (!(ext.parsedValue instanceof type) || !equal(new Uint8Array(
      (ext.parsedValue.toSchema?.() ?? ext.parsedValue).toBER(false)), ext.extnValue.valueBlock.valueHexView)) throw Error('extension encoding/type');
    if (ext.extnID === AKI && ('authorityCertIssuer' in ext.parsedValue || 'authorityCertSerialNumber' in ext.parsedValue)) throw Error('unsupported AKI form');
    extensions.set(ext.extnID, ext);
  }
  const bc = extensions.get(BC), ku = extensions.get(KU);
  if (!bc || !ku || !bc.critical || (ca && !ku.critical) || Boolean(bc.parsedValue.cA) !== ca) throw Error('CA constraints');
  const usage = ku.parsedValue.valueBlock;
  if (usage.valueHexView.length !== 1 || usage.unusedBits > 7
      || usage.unusedBits !== Math.min(7, Math.log2(usage.valueHexView[0] & -usage.valueHexView[0]))
      || (usage.valueHexView[0] & (ca ? 4 : 128)) === 0
      || (!ca && usage.valueHexView[0] & 6)) throw Error('key usage');
  if ('pathLenConstraint' in bc.parsedValue && (!ca || !Number.isSafeInteger(bc.parsedValue.pathLenConstraint)
      || bc.parsedValue.pathLenConstraint < 0)) throw Error('path length');
  if (ca && !extensions.get(SKI)) throw Error('CA SKI');
  const ski = extensions.get(SKI)?.parsedValue.valueBlock.valueHexView;
  if (ski && (ski.length !== 20 || !ski.some(v => v !== 0))) throw Error('SKI shape');
  // Real AWS intermediates include noncritical CRL distribution metadata. Like
  // the native verifier, this profile does not fetch/trust host-provided CRLs.
  for (const oid of [SKI, AKI, CRLDP]) {
    const ext = extensions.get(oid);
    if (ext?.critical) throw Error('identifier criticality');
  }
  return { cert, extensions, bc: bc.parsedValue };
}
export async function verifiedLeaf(d, trustedRoot, at) {
  if (!Number.isSafeInteger(at) || at < 0 || at > 8640000000000000) throw Error('trusted time');
  if (!equal(d.chain[0], trustedRoot)) throw Error('root');
  const path = [...d.chain, d.certificate].map((b, i, all) => certificate(b, at, i < all.length - 1));
  if (new Set([...d.chain, d.certificate].map(b => [...b].join(','))).size !== path.length) throw Error('repeated certificate');
  for (let i = 0; i < path.length - 1; i++) {
    const { cert, bc } = path[i], below = path.length - i - 2;
    if (bc.pathLenConstraint !== undefined && below > bc.pathLenConstraint) throw Error('CA path length');
    const child = path[i + 1];
    if (!child.cert.issuer.isEqual(cert.subject) || child.cert.issuer.isEqual(child.cert.subject)) throw Error('issuer order/self-issued child');
    const aki = child.extensions.get(AKI)?.parsedValue.keyIdentifier;
    // AWS NSM leaves omit AKI. Only that leaf may omit it; intermediates must
    // have a key identifier matching the exact selected issuer's SKI.
    if (i + 1 < path.length - 1 && !aki) throw Error('CA AKI');
    if (aki && !equal(aki.valueBlock.valueHexView, path[i].extensions.get(SKI).parsedValue.valueBlock.valueHexView)) throw Error('AKI mismatch');
    if (child.extensions.has(AKI) && !aki) throw Error('unsupported AKI form');
  }
  const root = path[0].cert;
  const engine = new CryptoEngine({ name: 'qualification-platform', crypto: globalThis.crypto, subtle: globalThis.crypto.subtle });
  if (!root.issuer.isEqual(root.subject) || !await root.verify(root, engine)) throw Error('root signature');
  const validator = new CertificateChainValidationEngine({
    trustedCerts: [root], certs: path.slice(1).map(p => p.cert), checkDate: new Date(at),
  });
  const result = await validator.verify({ passedWhenNotRevValues: true }, engine);
  if (!result.result || result.certificatePath?.length !== path.length) throw Error('PKIX path');
  return path.at(-1).cert;
}
