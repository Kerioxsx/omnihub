// Copy text with the Clipboard API when available (secure contexts), else
// the classic hidden-textarea + execCommand fallback that also works over
// plain HTTP and on older iOS.

export function canCopy(): boolean {
  if (navigator.clipboard && window.isSecureContext) return true;
  try {
    return typeof document.queryCommandSupported === 'function' ? document.queryCommandSupported('copy') : true;
  } catch {
    return false;
  }
}

export async function copyText(text: string): Promise<boolean> {
  if (navigator.clipboard && window.isSecureContext) {
    try {
      await navigator.clipboard.writeText(text);
      return true;
    } catch {
      /* fall through */
    }
  }
  const ta = document.createElement('textarea');
  ta.value = text;
  ta.setAttribute('readonly', '');
  ta.style.cssText = 'position:fixed;top:0;left:0;width:1px;height:1px;opacity:0;font-size:16px;';
  document.body.appendChild(ta);
  const active = document.activeElement as HTMLElement | null;
  ta.focus();
  ta.select();
  ta.setSelectionRange(0, text.length);
  let ok = false;
  try {
    ok = document.execCommand('copy');
  } catch {
    ok = false;
  }
  ta.remove();
  active?.focus?.();
  return ok;
}
