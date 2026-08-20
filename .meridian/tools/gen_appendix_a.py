#!/usr/bin/env python3
"""Regenerate Appendix A of MERIDIAN_SPECOMENT.md as a real traceability index.

Attribution rule: an identifier is OWNED by the heading that declares it (the
heading text contains the identifier). Every other occurrence is a reference.
Identifiers that appear only in body text and never in a heading are reported
separately as 'referenced but never declared' rather than being silently
attributed to whichever section mentioned them first.

Usage: gen_appendix_a.py <path-to-specoment>
"""
import re, sys, collections

ID_RE = re.compile(r'`([A-Z][A-Z0-9]*(?:-[A-Z0-9]+)*-(\d{3})(?:\.\.(\d{3}))?)`')
BARE_ID_RE = re.compile(r'(?<![A-Za-z0-9-])([A-Z][A-Z0-9]*(?:-[A-Z0-9]+)*-(\d{3})(?:\.\.(\d{3}))?)(?![0-9-])')
LEGACY = re.compile(r'^(WP-UI|WP-REL|WP-EDT|WP-BLD|WP-PRC|WP-MDL|WP-GAM|MS|RG|WVR|REQ|DEP|VAL|PRG)-')
HEAD_RE = re.compile(r'^(#{1,6})\s+(.*)$')

def expand(tok):
    """`FAM-001..005` -> [FAM-001..FAM-005]; single ids pass through."""
    m = re.match(r'^(.*?)-(\d{3})\.\.(\d{3})$', tok)
    if not m:
        return [tok]
    fam, lo, hi = m.group(1), int(m.group(2)), int(m.group(3))
    return [f'{fam}-{n:03d}' for n in range(lo, hi + 1)]

def family(i):
    return i.rsplit('-', 1)[0]

def main(path):
    lines = open(path, encoding='utf-8').read().split('\n')
    # section index: for each line, the nearest preceding heading of level<=4
    sections, cur = [], ('(preamble)', 0)
    heads = []
    for n, ln in enumerate(lines, 1):
        m = HEAD_RE.match(ln)
        if m and len(m.group(1)) <= 4:
            cur = (m.group(2).strip(), n)
            heads.append(cur)
        sections.append(cur)

    body_end = next(n for n, ln in enumerate(lines, 1) if ln.startswith('# Appendix A'))

    owner = {}          # id -> (heading, line)
    refs = collections.defaultdict(list)   # id -> [(heading,line)]
    seen_families = collections.defaultdict(set)
    dup_owner = collections.defaultdict(list)
    fam_owner = {}

    for n, ln in enumerate(lines[:body_end - 1], 1):
        head, hline = sections[n - 1]
        is_heading = bool(HEAD_RE.match(ln))
        # Gap analysis counts only declarations (headings) and backticked
        # references. A bare prose mention such as "AI-031 through AI-033" is
        # discussion, not an assignment, and must not mask a real gap.
        for m in (BARE_ID_RE if is_heading else ID_RE).finditer(ln):
            for i in expand(m.group(1)):
                seen_families[family(i)].add(int(i.rsplit('-', 1)[1]))
        scan = BARE_ID_RE if is_heading else ID_RE
        for m in scan.finditer(ln):
            toks = expand(m.group(1))
            is_range = len(toks) > 1
            for i in toks:
                if is_heading and not is_range:
                    if i in owner and owner[i][1] != hline:
                        dup_owner[i].append((head, hline))
                    else:
                        owner[i] = (head, hline)
                elif is_heading and is_range:
                    fam_owner.setdefault(i, (head, hline))
                else:
                    if (head, hline) not in refs[i]:
                        refs[i].append((head, hline))

    all_ids = sorted(set(list(owner) + list(refs)),
                     key=lambda i: (family(i), int(i.rsplit('-', 1)[1])))

    out = []
    out.append('This index is generated from the canonical body by '
               '`gen_appendix_a.py`. An identifier is listed against the heading that '
               '**declares** it; every other occurrence is listed as a reference. '
               'Identifiers that never appear in a heading are reported under '
               '*Referenced but never declared* instead of being attributed to their '
               'first mention. Typed repository registries MUST expand identifier '
               'families into individual records and preserve zero-unmapped '
               'traceability when this document is split.')
    out.append('')
    out.append('## A.1 Declared identifiers')
    out.append('')
    for i, v in fam_owner.items():
        owner.setdefault(i, v)
    undeclared, legacy = [], []
    for i in all_ids:
        if i not in owner:
            (legacy if LEGACY.match(i) else undeclared).append(i)
            continue
        h, l = owner[i]
        extra = refs.get(i, [])
        line = f'- `{i}` — owned by *{h}* (line {l})'
        if extra:
            names = '; '.join(f'*{x[0]}*' for x in extra[:3])
            more = f'; +{len(extra)-3} more' if len(extra) > 3 else ''
            line += f' — also referenced in: {names}{more}'
        out.append(line)

    out.append('')
    out.append('## A.2 Referenced but never declared')
    out.append('')
    if undeclared:
        out.append('These identifiers are cited in the body but no heading declares '
                   'them. Each MUST either receive an owning contract or be removed '
                   'before `PH-AUTH-002` can claim that every canonical identifier is '
                   'indexable exactly once.')
        out.append('')
        for i in undeclared:
            where = '; '.join(f'*{x[0]}* (line {x[1]})' for x in refs[i][:4])
            out.append(f'- `{i}` — cited in: {where}')
    else:
        out.append('None. Every cited identifier has an owning heading.')

    out.append('')
    out.append('## A.2b Retired v0.5 identifiers cited as history')
    out.append('')
    out.append('These identifiers belong to the frozen v0.5 authority. They appear only '
               'as migration/disposition history and MUST NOT be treated as live '
               'contracts or re-entered into v1 registries.')
    out.append('')
    for i in legacy:
        where = '; '.join(f'*{x[0]}* (line {x[1]})' for x in refs[i][:3])
        out.append(f'- `{i}` — cited in: {where}')

    out.append('')
    out.append('## A.3 Identifiers declared by more than one heading')
    out.append('')
    if dup_owner:
        for i, extras in sorted(dup_owner.items()):
            first = owner[i]
            rest = '; '.join(f'*{h}* (line {l})' for h, l in extras)
            out.append(f'- `{i}` — declared by *{first[0]}* (line {first[1]}) and {rest}')
    else:
        out.append('None. No identifier is declared by two headings.')

    out.append('')
    out.append('## A.4 Identifier gaps and their provenance disposition')
    out.append('')
    out.append('These numbers fall inside an otherwise contiguous family but carry no '
               'contract in this document. A gap is **not** self-justifying: each one is '
               'either a deliberate reservation, an identifier whose substance was '
               'absorbed by a later contract, or genuine synthesis loss that must be '
               'restored from the planning ledger. The dispositions below are recorded '
               'findings, not a blanket exemption.')
    out.append('')
    out.append('| Gap | Disposition | Basis |')
    out.append('|---|---|---|')
    for row in [
        ('`AI-005..026`', 'Restored', 'Locked in planning ledger v0.66; identifiers reattached to their surviving contracts in the AI section.'),
        ('`AI-027..030`', 'Restored', 'Locked in Meridian AI planning ledger v0.1 and matching AI-POLICY-001..004 one-to-one: model-neutral harness first, scoped cloud authorization, portable-vs-private memory split, first-party model channel.'),
        ('`AI-031..033`', 'No provenance found', 'AI-POLICY-005..008 answer the owner questions that ledger v0.1 left open after Round 1 and carry no recovered identity. Not invented to close the sequence; AI-034/035 legitimately follow a reserved gap. Re-check against AI ledger v0.6 if it surfaces.'),
        ('`SRV-011..016`', 'Restored', 'Locked in planning ledger v0.66. Prose for 011-015 survived without identifiers; 016 rejoins the deferred orchestrator family.'),
        ('`DIST-005`', 'Restored', 'Locked in planning ledger v0.66 as *Breadth without fake support*; reinstated and reconciled with the 1.0 platform floor.'),
        ('`SECURITY-001`, `SECURITY-002`', 'Absorbed; do not resurrect', 'Substance reorganized into the `PROTECT-*` contract. A second security namespace is not reopened.'),
        ('`REUSE-001..006`', 'Absorbed; do not resurrect', 'Superseded by `RESEARCH-005`, `RESEARCH-007/008` and the `LEGAL-*` intake/relicensing rules. No evidence of 001-005 in the final ledger.'),
        ('`MOD-001`', 'No evidence; unassigned', 'No locked `MOD-001` found in the final ledger. Leading gap left open pending stronger provenance.'),
        ('`ISO-004`, `ISO-005`', 'Absorbed; do not resurrect', 'Ledger v0.66 records them as historical stepping stones explicitly superseded by `ISO-007` through `ISO-011`.'),
        ('`RESEARCH-006`', 'Restored', 'Locked in ledger v0.66 as *Source-grounded planning before owner questions*; reinstated with an owning contract.'),
        ('`TWO-001..003`, `GOV-COVERAGE-001`, `MOD-001`', 'No provenance found', 'Absent from every ledger on this machine: spec-rewrite v0.22, v0.26, v0.53, v0.57, v0.61, v0.66, the AI ledger v0.1, and the amendment prompt. Left unassigned rather than invented.'),
    ]:
        out.append(f'| {row[0]} | {row[1]} | {row[2]} |')
    out.append('')
    out.append('Residual gaps detected by this run:')
    out.append('')
    known = {family(i) for i in list(owner) + list(refs)}
    gaps_found = False
    for fam in sorted(f for f in seen_families if f in known):
        nums = seen_families[fam]
        if not nums:
            continue
        if LEGACY.match(fam + '-001'):
            continue
        missing = [n for n in range(1, max(nums) + 1) if n not in nums]
        if missing:
            gaps_found = True
            out.append(f'- `{fam}`: ' + ', '.join(f'`{fam}-{n:03d}`' for n in missing))
    if not gaps_found:
        out.append('- None.')

    out.append('')
    out.append(f'**Index totals:** {len(owner)} declared, {len(undeclared)} undeclared, '
               f'{len(dup_owner)} multiply-declared, {len(legacy)} retired-v0.5, '
               f'{len(seen_families)} identifier families.')
    return '\n'.join(out)

if __name__ == '__main__':
    print(main(sys.argv[1]))
