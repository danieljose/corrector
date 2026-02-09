#!/usr/bin/env python3
"""
Build Spanish dictionary by merging:
- Existing curated dictionary (data/es/words.txt) with highest priority
- FreeLing lexical entries (MM.*) as morphological base
- doozan es_merged_50k frequencies

Output format:
forma|categoria|genero|numero|lema|frecuencia
"""

from __future__ import annotations

import argparse
import datetime as dt
import json
import math
import shutil
import sys
import urllib.error
import urllib.request
from dataclasses import dataclass
from pathlib import Path
from typing import Dict, Iterable, List, Optional


FREELING_API = "https://api.github.com/repos/TALP-UPC/FreeLing/contents/data/es/dictionary/entries"
DOOZAN_RAW = "https://raw.githubusercontent.com/doozan/spanish_data/{ref}/es_merged_50k.txt"


CATEGORY_PRIORITY = {
    "sustantivo": 1,
    "adjetivo": 2,
    "adverbio": 3,
    "articulo": 4,
    "determinante": 5,
    "pronombre": 6,
    "preposicion": 7,
    "conjuncion": 8,
    "verbo": 9,
    "otro": 10,
}


@dataclass
class Entry:
    word: str
    category: str
    gender: str
    number: str
    lemma: str
    frequency: int
    source: str  # current|freeling


@dataclass
class Candidate:
    word: str
    lemma: str
    category: str
    gender: str
    number: str
    tag: str
    source_file: str


def eprint(msg: str) -> None:
    print(msg, file=sys.stderr)


def fetch_bytes(url: str, timeout: int = 60) -> bytes:
    req = urllib.request.Request(
        url,
        headers={"User-Agent": "corrector-build-es-dictionary/1.0"},
    )
    with urllib.request.urlopen(req, timeout=timeout) as response:
        return response.read()


def decode_bytes(data: bytes) -> str:
    try:
        return data.decode("utf-8")
    except UnicodeDecodeError:
        return data.decode("latin-1")


def fetch_json(url: str) -> object:
    return json.loads(decode_bytes(fetch_bytes(url)))


def ensure_dir(path: Path) -> None:
    path.mkdir(parents=True, exist_ok=True)


def char_at(text: str, idx: int) -> str:
    return text[idx] if idx < len(text) else ""


def map_gender(c: str) -> str:
    c = c.upper()
    if c == "M":
        return "m"
    if c == "F":
        return "f"
    return "_"


def map_number(c: str) -> str:
    c = c.upper()
    if c == "S":
        return "s"
    if c == "P":
        return "p"
    return "_"


def parse_freeling_candidate(
    form: str, lemma: str, tag: str, source_file: str, stats: Dict[str, int]
) -> Optional[Candidate]:
    form = form.strip().lower()
    lemma = lemma.strip().lower()
    tag = tag.strip()

    if not form or not tag or any(ch.isspace() for ch in form):
        return None

    # Skip proper nouns. Project uses names.txt for those.
    if tag.startswith("NP"):
        stats["freeling_skipped_np"] += 1
        return None

    category = None
    gender = "_"
    number = "_"

    if tag.startswith("NC"):
        category = "sustantivo"
        gender = map_gender(char_at(tag, 2))
        number = map_number(char_at(tag, 3))
    elif tag.startswith("AQ") or tag.startswith("AO"):
        category = "adjetivo"
        gender = map_gender(char_at(tag, 3))
        number = map_number(char_at(tag, 4))
    elif tag.startswith("DA"):
        category = "articulo"
        gender = map_gender(char_at(tag, 3))
        number = map_number(char_at(tag, 4))
    elif (
        tag.startswith("DD")
        or tag.startswith("DI")
        or tag.startswith("DP")
        or tag.startswith("DT")
        or tag.startswith("DE")
    ):
        category = "determinante"
        gender = map_gender(char_at(tag, 3))
        number = map_number(char_at(tag, 4))
    elif (
        tag.startswith("PP")
        or tag.startswith("PD")
        or tag.startswith("PR")
        or tag.startswith("PT")
        or tag.startswith("PI")
        or tag.startswith("PE")
    ):
        category = "pronombre"
        gender = map_gender(char_at(tag, 3))
        number = map_number(char_at(tag, 4))
    elif tag.startswith("SP"):
        category = "preposicion"
    elif tag.startswith("CC") or tag.startswith("CS"):
        category = "conjuncion"
    elif tag.startswith("RG") or tag.startswith("RN"):
        category = "adverbio"
    elif tag.startswith("I"):
        category = "otro"
    elif tag.startswith("V"):
        # Keep only infinitives as verbs.
        # Convert participles to adjectives with gender/number.
        mood = char_at(tag, 2)
        if mood == "N":
            category = "verbo"
            stats["freeling_infinitive_candidates"] += 1
        elif mood == "P":
            category = "adjetivo"
            # Example VMP00SF / VAP00SM -> number at index 5, gender at index 6
            number = map_number(char_at(tag, 5))
            gender = map_gender(char_at(tag, 6))
            stats["freeling_participle_candidates"] += 1
        else:
            stats["freeling_skipped_conjugated_verbs"] += 1
            return None

    if category is None:
        stats["freeling_skipped_unknown_tag"] += 1
        return None

    return Candidate(
        word=form,
        lemma=lemma,
        category=category,
        gender=gender,
        number=number,
        tag=tag,
        source_file=source_file,
    )


def select_best_candidate(candidates: List[Candidate]) -> Candidate:
    def score(c: Candidate) -> tuple:
        specificity = (1 if c.gender != "_" else 0) + (1 if c.number != "_" else 0)
        return (
            CATEGORY_PRIORITY.get(c.category, 999),
            -specificity,
            c.category,
            c.gender,
            c.number,
            c.lemma,
            c.tag,
        )

    return min(candidates, key=score)


def parse_current_dictionary(path: Path, stats: Dict[str, int]) -> Dict[str, Entry]:
    current: Dict[str, Entry] = {}
    with path.open("r", encoding="utf-8") as f:
        for line in f:
            raw = line.rstrip("\n")
            line = raw.strip()
            if not line or line.startswith("#"):
                continue

            parts = line.split("|", 5)
            word = parts[0].strip().lower() if len(parts) > 0 else ""
            if not word:
                continue

            category = parts[1].strip() if len(parts) > 1 and parts[1].strip() else "otro"
            gender = parts[2].strip() if len(parts) > 2 and parts[2].strip() else "_"
            number = parts[3].strip() if len(parts) > 3 and parts[3].strip() else "_"
            lemma = parts[4].strip() if len(parts) > 4 else ""

            frequency = 1
            if len(parts) > 5:
                try:
                    frequency = max(1, int(parts[5].strip()))
                except ValueError:
                    frequency = 1

            entry = Entry(
                word=word,
                category=category,
                gender=gender,
                number=number,
                lemma=lemma,
                frequency=frequency,
                source="current",
            )

            if word in current:
                stats["current_duplicates"] += 1
                # Mirror Trie behavior: keep duplicate with higher frequency.
                if entry.frequency > current[word].frequency:
                    current[word] = entry
                continue

            current[word] = entry

    return current


def fetch_freeling_entry_urls(ref: str) -> Dict[str, str]:
    api_url = f"{FREELING_API}?ref={ref}"
    payload = fetch_json(api_url)
    if not isinstance(payload, list):
        raise RuntimeError(f"Unexpected FreeLing API response: {type(payload).__name__}")

    urls: Dict[str, str] = {}
    for item in payload:
        if not isinstance(item, dict):
            continue
        name = str(item.get("name", ""))
        if not name.startswith("MM."):
            continue
        download_url = str(item.get("download_url", ""))
        if not download_url:
            continue
        urls[name] = download_url

    if not urls:
        raise RuntimeError("No MM.* files found in FreeLing entries directory")
    return dict(sorted(urls.items()))


def download_to_cache(url: str, target: Path, force: bool = False) -> None:
    if target.exists() and not force:
        return
    target.write_bytes(fetch_bytes(url))


def parse_freeling_files(
    files: Dict[str, Path], stats: Dict[str, int]
) -> Dict[str, List[Candidate]]:
    by_word: Dict[str, List[Candidate]] = {}
    for name, path in files.items():
        data = decode_bytes(path.read_bytes())
        for raw_line in data.splitlines():
            line = raw_line.strip()
            if not line or line.startswith("#"):
                continue
            parts = line.split()
            if len(parts) < 3:
                stats["freeling_bad_lines"] += 1
                continue
            form, lemma, tag = parts[0], parts[1], parts[2]
            cand = parse_freeling_candidate(form, lemma, tag, name, stats)
            if cand is None:
                continue
            by_word.setdefault(cand.word, []).append(cand)
            stats["freeling_kept_candidates"] += 1
    return by_word


def load_doozan_frequency(
    path: Path, stats: Dict[str, int]
) -> tuple[Dict[str, int], Dict[str, int]]:
    raw_freq: Dict[str, int] = {}
    max_raw = 0

    with path.open("r", encoding="utf-8") as f:
        for raw in f:
            line = raw.strip()
            if not line or line.startswith("#"):
                continue

            if "\t" in line:
                word, count_text = line.split("\t", 1)
            else:
                parts = line.split()
                if len(parts) < 2:
                    stats["doozan_bad_lines"] += 1
                    continue
                word, count_text = parts[0], parts[1]

            word = word.strip().lower()
            if not word:
                continue

            try:
                count = int(count_text.strip())
            except ValueError:
                stats["doozan_bad_lines"] += 1
                continue

            if count <= 0:
                continue

            prev = raw_freq.get(word)
            if prev is None or count > prev:
                raw_freq[word] = count
                if count > max_raw:
                    max_raw = count

    if max_raw <= 0:
        raise RuntimeError("doozan frequency file had no positive counts")

    stats["doozan_max_raw"] = max_raw

    denom = math.log10(max_raw)
    normalized: Dict[str, int] = {}
    for word, count in raw_freq.items():
        if count <= 1:
            norm = 1
        else:
            value = int((math.log10(count) / denom) * 1000)
            norm = max(1, min(1000, value))
        normalized[word] = norm
    return normalized, raw_freq


def is_probable_unaccented_verb_variant(
    word: str, category: str, current: Dict[str, Entry], doozan_raw: Dict[str, int]
) -> bool:
    if category not in {"sustantivo", "adjetivo", "adverbio", "otro"}:
        return False
    if any(ch in "áéíóú" for ch in word):
        return False

    accent_map = {"a": "á", "e": "é", "i": "í", "o": "ó", "u": "ú"}
    raw_word = doozan_raw.get(word, 0)

    for idx, ch in enumerate(word):
        repl = accent_map.get(ch)
        if repl is None:
            continue
        variant = word[:idx] + repl + word[idx + 1 :]
        entry = current.get(variant)
        if entry is None or entry.category != "verbo":
            continue

        raw_variant = doozan_raw.get(variant, 0)
        if raw_variant >= max(2000, raw_word * 5):
            return True

    return False


def merge_entries(
    current: Dict[str, Entry],
    freeling_candidates: Dict[str, List[Candidate]],
    doozan_vocab: set[str],
    doozan_raw: Dict[str, int],
    stats: Dict[str, int],
) -> Dict[str, Entry]:
    final_entries: Dict[str, Entry] = dict(current)

    for word, candidates in freeling_candidates.items():
        best = select_best_candidate(candidates)
        stats["freeling_unique_forms"] += 1

        if word in final_entries:
            stats["conflicts_with_current"] += 1
            # Preserve current category/gender/number. Fill lemma only if missing.
            if not final_entries[word].lemma and best.lemma:
                final_entries[word].lemma = best.lemma
                stats["filled_current_lemmas_from_freeling"] += 1
            continue

        # Keep new infinitives only when frequency-backed by doozan.
        # This reduces false verb positives from very rare lemmas.
        if best.category == "verbo" and word not in doozan_vocab:
            stats["skipped_infinitive_not_in_doozan"] += 1
            continue

        if is_probable_unaccented_verb_variant(
            word, best.category, current, doozan_raw
        ):
            stats["skipped_probable_unaccented_typos"] += 1
            continue

        final_entries[word] = Entry(
            word=word,
            category=best.category,
            gender=best.gender,
            number=best.number,
            lemma=best.lemma,
            frequency=1,
            source="freeling",
        )
        stats["added_from_freeling"] += 1

    return final_entries


def assign_frequencies(
    entries: Dict[str, Entry], doozan_freq: Dict[str, int], stats: Dict[str, int]
) -> None:
    for word, entry in entries.items():
        freq = doozan_freq.get(word)
        if freq is not None:
            entry.frequency = freq
            stats["freq_from_doozan"] += 1
        elif entry.source == "freeling":
            entry.frequency = 1
            stats["freq_missing_freeling_to_1"] += 1
        else:
            # Preserve existing curated frequency for current dictionary words.
            entry.frequency = max(1, entry.frequency)
            stats["freq_kept_current"] += 1


def validate(entries: Dict[str, Entry]) -> None:
    checks = [
        ("agua", "sustantivo", "f", None),
        ("casa", "sustantivo", None, None),
        ("el", "articulo", "m", "s"),
    ]
    for word, exp_cat, exp_gen, exp_num in checks:
        if word not in entries:
            raise RuntimeError(f"Validation failed: missing required word '{word}'")
        e = entries[word]
        if exp_cat is not None and e.category != exp_cat:
            raise RuntimeError(
                f"Validation failed: '{word}' category '{e.category}' != '{exp_cat}'"
            )
        if exp_gen is not None and e.gender != exp_gen:
            raise RuntimeError(
                f"Validation failed: '{word}' gender '{e.gender}' != '{exp_gen}'"
            )
        if exp_num is not None and e.number != exp_num:
            raise RuntimeError(
                f"Validation failed: '{word}' number '{e.number}' != '{exp_num}'"
            )


def write_output(path: Path, entries: Dict[str, Entry], no_backup: bool) -> Optional[Path]:
    backup_path: Optional[Path] = None
    if path.exists() and not no_backup:
        backup = Path(f"{path}.bak")
        if backup.exists():
            stamp = dt.datetime.now(dt.timezone.utc).strftime("%Y%m%d%H%M%S")
            backup = Path(f"{path}.{stamp}.bak")
        shutil.copy2(path, backup)
        backup_path = backup

    now = dt.datetime.now(dt.timezone.utc).isoformat()
    with path.open("w", encoding="utf-8", newline="\n") as f:
        f.write("# Diccionario de espanol combinado (generado automaticamente)\n")
        f.write("# Formato: palabra|categoria|genero|numero|lema|frecuencia\n")
        f.write(f"# Generado UTC: {now}\n")
        f.write("\n")

        for word in sorted(entries.keys()):
            e = entries[word]
            lemma = e.lemma if e.lemma else ""
            line = (
                f"{e.word}|{e.category}|{e.gender}|{e.number}|"
                f"{lemma}|{max(1, int(e.frequency))}"
            )
            f.write(line + "\n")
    return backup_path


def print_stats(stats: Dict[str, int], current_count: int, final_count: int) -> None:
    print("=== build_es_dictionary stats ===")
    print(f"current_entries: {current_count}")
    print(f"freeling_files: {stats.get('freeling_files', 0)}")
    print(f"freeling_kept_candidates: {stats.get('freeling_kept_candidates', 0)}")
    print(f"freeling_unique_forms: {stats.get('freeling_unique_forms', 0)}")
    print(f"freeling_infinitive_candidates: {stats.get('freeling_infinitive_candidates', 0)}")
    print(f"freeling_participle_candidates: {stats.get('freeling_participle_candidates', 0)}")
    print(
        f"freeling_skipped_conjugated_verbs: {stats.get('freeling_skipped_conjugated_verbs', 0)}"
    )
    print(f"freeling_skipped_np: {stats.get('freeling_skipped_np', 0)}")
    print(
        "skipped_infinitive_not_in_doozan: "
        f"{stats.get('skipped_infinitive_not_in_doozan', 0)}"
    )
    print(
        "skipped_probable_unaccented_typos: "
        f"{stats.get('skipped_probable_unaccented_typos', 0)}"
    )
    print(f"conflicts_with_current: {stats.get('conflicts_with_current', 0)}")
    print(f"added_from_freeling: {stats.get('added_from_freeling', 0)}")
    print(
        "filled_current_lemmas_from_freeling: "
        f"{stats.get('filled_current_lemmas_from_freeling', 0)}"
    )
    print(f"doozan_max_raw: {stats.get('doozan_max_raw', 0)}")
    print(f"freq_from_doozan: {stats.get('freq_from_doozan', 0)}")
    print(f"freq_missing_freeling_to_1: {stats.get('freq_missing_freeling_to_1', 0)}")
    print(f"freq_kept_current: {stats.get('freq_kept_current', 0)}")
    print(f"current_duplicates_ignored: {stats.get('current_duplicates', 0)}")
    print(f"final_entries: {final_count}")


def build(args: argparse.Namespace) -> int:
    stats: Dict[str, int] = {
        "freeling_files": 0,
        "freeling_kept_candidates": 0,
        "freeling_unique_forms": 0,
        "freeling_infinitive_candidates": 0,
        "freeling_participle_candidates": 0,
        "freeling_skipped_conjugated_verbs": 0,
        "freeling_skipped_unknown_tag": 0,
        "freeling_skipped_np": 0,
        "freeling_bad_lines": 0,
        "doozan_bad_lines": 0,
        "conflicts_with_current": 0,
        "added_from_freeling": 0,
        "filled_current_lemmas_from_freeling": 0,
        "freq_from_doozan": 0,
        "freq_missing_freeling_to_1": 0,
        "freq_kept_current": 0,
        "current_duplicates": 0,
        "skipped_infinitive_not_in_doozan": 0,
        "skipped_probable_unaccented_typos": 0,
    }

    current_path = Path(args.current).resolve()
    output_path = Path(args.output).resolve()
    cache_dir = Path(args.cache_dir).resolve()
    ensure_dir(cache_dir)

    # 1) Discover and download FreeLing files
    freeling_urls = fetch_freeling_entry_urls(args.freeling_ref)
    stats["freeling_files"] = len(freeling_urls)
    freeling_files: Dict[str, Path] = {}
    for name, url in freeling_urls.items():
        target = cache_dir / name
        download_to_cache(url, target, force=args.force_download)
        freeling_files[name] = target

    # 2) Download doozan frequencies
    doozan_url = DOOZAN_RAW.format(ref=args.doozan_ref)
    doozan_path = cache_dir / "es_merged_50k.txt"
    download_to_cache(doozan_url, doozan_path, force=args.force_download)

    # 3) Load current dictionary
    current = parse_current_dictionary(current_path, stats)

    # 4) Frequencies (needed before merge to filter rare infinitives)
    doozan_freq, doozan_raw = load_doozan_frequency(doozan_path, stats)
    doozan_vocab = set(doozan_freq.keys())

    # 5) Parse FreeLing + resolve ambiguity
    freeling_candidates = parse_freeling_files(freeling_files, stats)

    # 6) Merge (current wins)
    merged = merge_entries(current, freeling_candidates, doozan_vocab, doozan_raw, stats)
    assign_frequencies(merged, doozan_freq, stats)

    # 7) Validation
    validate(merged)

    # 8) Write output + backup
    if args.dry_run:
        print("Dry run enabled: output file not written.")
    else:
        backup = write_output(output_path, merged, no_backup=args.no_backup)
        print(f"Wrote dictionary: {output_path}")
        if backup is not None:
            print(f"Backup path: {backup}")

    print_stats(stats, current_count=len(current), final_count=len(merged))
    return 0


def parse_args(argv: Optional[Iterable[str]] = None) -> argparse.Namespace:
    p = argparse.ArgumentParser(
        description="Build merged Spanish dictionary from FreeLing + doozan + current words.txt"
    )
    p.add_argument("--current", default="data/es/words.txt", help="Current dictionary path")
    p.add_argument("--output", default="data/es/words.txt", help="Output dictionary path")
    p.add_argument(
        "--cache-dir",
        default=".cache/es-dictionary",
        help="Download cache directory",
    )
    p.add_argument("--freeling-ref", default="master", help="FreeLing git ref (branch/tag/sha)")
    p.add_argument("--doozan-ref", default="master", help="doozan git ref (branch/tag/sha)")
    p.add_argument("--force-download", action="store_true", help="Re-download all sources")
    p.add_argument("--dry-run", action="store_true", help="Run full build but do not write output")
    p.add_argument("--no-backup", action="store_true", help="Do not create .bak backup")
    return p.parse_args(argv)


def main(argv: Optional[Iterable[str]] = None) -> int:
    args = parse_args(argv)
    try:
        return build(args)
    except urllib.error.URLError as exc:
        eprint(f"Network error: {exc}")
        return 2
    except Exception as exc:  # pylint: disable=broad-except
        eprint(f"Error: {exc}")
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
