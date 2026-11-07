"""Exact security-output exceptions for pinned-reference comparisons."""


def diagnostic_escape(data):
    """Independently render untrusted fixture bytes for terminal-safe output."""
    out = bytearray()
    for char in data.decode('utf-8', 'surrogateescape'):
        code = ord(char)
        if 0xdc80 <= code <= 0xdcff:
            out.extend(f'\\{code-0xdc00:03o}'.encode())
        elif code < 0x20 or 0x7f <= code <= 0x9f or code in (0xad, 0x61c, 0xfeff) or 0x200b <= code <= 0x200f or 0x2028 <= code <= 0x202e or 0x2060 <= code <= 0x206f:
            out.extend(f'\\u{{{code:x}}}'.encode())
        else:
            out.extend(char.encode())
    return bytes(out)


def locale_security_escape_only(row, tokens):
    """Accept only an exact escape of known hostile filename tokens."""
    reference, rust = row['reference'], row['rust']
    if reference['status'] != rust['status'] or reference['stderr_hex'] != rust['stderr_hex']:
        return False
    expected = bytes.fromhex(reference['stdout_hex'])
    for token in tokens:
        expected = expected.replace(token, diagnostic_escape(token))
    return expected != bytes.fromhex(reference['stdout_hex']) and expected == bytes.fromhex(rust['stdout_hex'])


def database_security_escape_only(label, action, reference, rust):
    """Accept only the exact CR/tab escapes in the two database fixtures."""
    expected = reference.copy()
    if label == 'crlf':
        raw, safe, field = b'bogus x bad\r', b'bogus x bad\\u{d}', 'stderr_hex'
    elif label == 'compiled-preference' and action == 'l':
        raw, safe, field = b'\t', b'\\u{9}', 'stdout_hex'
    else:
        return False
    data = bytes.fromhex(reference[field])
    if raw not in data:
        return False
    expected[field] = data.replace(raw, safe).hex()
    return all(expected == observed for observed in rust)
