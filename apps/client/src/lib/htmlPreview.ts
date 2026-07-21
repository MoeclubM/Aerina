const STORAGE_PREFIX = "aerina.htmlPreview.";

export function saveHtmlPreview(source: string): string {
  const id = crypto.randomUUID();
  localStorage.setItem(`${STORAGE_PREFIX}${id}`, source);
  return id;
}

export function loadHtmlPreview(id: string): string | null {
  return localStorage.getItem(`${STORAGE_PREFIX}${id}`);
}

export function removeHtmlPreview(id: string) {
  localStorage.removeItem(`${STORAGE_PREFIX}${id}`);
}
