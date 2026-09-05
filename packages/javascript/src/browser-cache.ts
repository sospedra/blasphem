import { fail, type ManifestFile } from "./core/index.js";

interface StoredFile {
  bytes: Uint8Array;
  sha256: string;
}

const flights = new Map<string, Promise<Uint8Array>>();
let database: Promise<IDBDatabase> | undefined;

function openDatabase(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open("blasphem-assets", 1);
    request.onupgradeneeded = () => request.result.createObjectStore("files");
    request.onsuccess = () => {
      request.result.onversionchange = () => { request.result.close(); database = undefined; };
      resolve(request.result);
    };
    request.onerror = () => reject(request.error);
    request.onblocked = () => reject(new Error("Blasphem storage upgrade is blocked"));
  });
}

function cache(): Promise<IDBDatabase> {
  database ??= openDatabase().catch((error: unknown) => { database = undefined; throw error; });
  return database;
}

async function stored(key: string): Promise<StoredFile | undefined> {
  const db = await cache();
  return new Promise((resolve, reject) => {
    const request = db.transaction("files").objectStore("files").get(key);
    request.onsuccess = () => resolve(request.result as StoredFile | undefined);
    request.onerror = () => reject(request.error);
  });
}

async function commit(key: string, value: StoredFile): Promise<void> {
  const db = await cache();
  return new Promise((resolve, reject) => {
    const transaction = db.transaction("files", "readwrite");
    transaction.objectStore("files").put(value, key);
    transaction.oncomplete = () => resolve();
    transaction.onerror = () => reject(transaction.error);
    transaction.onabort = () => reject(transaction.error ?? new Error("Blasphem storage transaction aborted"));
  });
}

async function sha256(bytes: Uint8Array): Promise<string> {
  const digest = await crypto.subtle.digest("SHA-256", bytes as Uint8Array<ArrayBuffer>);
  return Array.from(new Uint8Array(digest), (byte) => byte.toString(16).padStart(2, "0")).join("");
}

async function cachedBytes(key: string): Promise<Uint8Array | undefined> {
  const record = await stored(key);
  if (!(record?.bytes instanceof Uint8Array)) return undefined;
  if (await sha256(record.bytes) !== record.sha256) return undefined;
  return record.bytes;
}

export async function fetchBytes(url: string): Promise<Uint8Array> {
  const response = await fetch(url, { signal: AbortSignal.timeout(30_000), cache: "no-store" });
  if (!response.ok) throw new Error(`${url} answered ${response.status}`);
  if (url.startsWith("https:") && response.url && !response.url.startsWith("https:")) {
    throw new Error("An HTTPS asset redirected to an insecure URL");
  }
  return new Uint8Array(await response.arrayBuffer());
}

async function twoAttempts(operation: () => Promise<Uint8Array>): Promise<Uint8Array> {
  try { return await operation(); }
  catch { return operation(); }
}

function share(key: string, operation: () => Promise<Uint8Array>): Promise<Uint8Array> {
  const existing = flights.get(key);
  if (existing) return existing;
  const flight = operation().finally(() => { flights.delete(key); });
  flights.set(key, flight);
  return flight;
}

function remoteKey(url: string, version: string): string {
  if (new URL(url).protocol !== "https:") throw fail("BLASPHEM_ASSETS_REQUIRED", "Remote assets require HTTPS");
  return `${version}:1:${url}`;
}

async function validFile(bytes: Uint8Array, expected: ManifestFile): Promise<boolean> {
  return bytes.byteLength === expected.bytes && await sha256(bytes) === expected.sha256;
}

async function fetchVerified(url: string, expected: ManifestFile): Promise<Uint8Array> {
  const bytes = await fetchBytes(url);
  if (!await validFile(bytes, expected)) throw fail("BLASPHEM_DIGEST_MISMATCH", `Integrity mismatch for ${url}`);
  return bytes;
}

export function readRemoteFile(url: string, release: { version: string; expected: ManifestFile }): Promise<Uint8Array> {
  const { version, expected } = release;
  const key = `${remoteKey(url, version)}:${expected.bytes}:${expected.sha256}`;
  return share(key, async () => {
    const cached = await cachedBytes(key);
    if (cached && await validFile(cached, expected)) return cached;
    const bytes = await twoAttempts(() => fetchVerified(url, expected));
    await commit(key, { bytes, sha256: expected.sha256 });
    return bytes;
  });
}
