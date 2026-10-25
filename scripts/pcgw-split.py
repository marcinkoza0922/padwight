#!/usr/bin/env python3
"""Split a PCGamingWiki XML dump into one wikitext file per page plus lookup tables.

Usage: python3 -I scripts/pcgw-split.py pcgamingwiki/raw/<dump>-current.xml pcgamingwiki

Writes, under the output directory:
  pages/<bucket>/<title>.wiki  one article each, entities decoded, with a short header
  games.tsv                    one row per article: ids, credits, the controller facts
  games.jsonl                  the same articles with every filled Input/API/Video/Audio
                               field, store rows and config/save paths, refs stripped
  steam-appids.tsv             Steam app id -> title and page file
  topics.tsv                   articles that aren't games (hardware, stores, tools, guides)
  redirects.tsv                redirect title -> target title
"""

import json
import os
import re
import sys
import xml.etree.ElementTree as ET

NS = "{http://www.mediawiki.org/xml/export-0.11/}"
SITE = "https://www.pcgamingwiki.com/wiki/"

# Templates whose named fields go to games.jsonl as they stand.
SECTIONS = {
    "input": "Input",
    "api": "API",
    "video": "Video",
    "audio": "Audio",
    "middleware": "Middleware",
    "cloud_saves": "Save game cloud syncing",
}

TSV_COLUMNS = [
    "title", "file", "steam_appids", "gog_ids", "developers", "publishers", "engines",
    "series", "genres", "released", "key_remap", "controller_support", "full_controller",
    "controller_remap", "xinput", "playstation", "nintendo", "steam_input_api",
    "steam_hook_input", "linux_native",
]
INPUT_COLUMNS = {
    "key_remap": "key remap",
    "controller_support": "controller support",
    "full_controller": "full controller",
    "controller_remap": "controller remap",
    "xinput": "xinput controllers",
    "playstation": "playstation controllers",
    "nintendo": "nintendo controllers",
    "steam_input_api": "steam input api",
    "steam_hook_input": "steam hook input",
}


BRACES = re.compile(r"\{\{|\}\}")


def templates(text):
    """Yields (name, body) for every {{...}} in text, nested ones included."""
    stack = []
    for brace in BRACES.finditer(text):
        if brace.group() == "{{":
            stack.append(brace.end())
        elif stack:
            body = text[stack.pop():brace.start()]
            yield body.split("|", 1)[0].strip(), body


def split_params(body):
    """Splits a template body on top-level pipes; returns (positional, named)."""
    parts, depth, cur = [], 0, []
    i = 0
    while i < len(body):
        two = body[i:i + 2]
        if two in ("{{", "[["):
            depth += 1
            cur.append(two)
            i += 2
            continue
        if two in ("}}", "]]"):
            depth -= 1
            cur.append(two)
            i += 2
            continue
        if body[i] == "|" and depth == 0:
            parts.append("".join(cur))
            cur = []
        else:
            cur.append(body[i])
        i += 1
    parts.append("".join(cur))
    positional, named = [], {}
    for part in parts[1:]:
        key, eq, value = part.partition("=")
        if eq and re.fullmatch(r"[\w \-]+", key.strip()):
            named[key.strip().lower()] = value.strip()
        else:
            positional.append(part.strip())
    return positional, named


REF = re.compile(r"<ref[^>/]*/>|<ref[^>]*>.*?</ref>", re.S)
PATH = re.compile(r"\{\{[pP]\|([^{}|]*)\}\}")
WRAP = re.compile(r"\{\{(?:file|folder|code|key|term|state|note|lang)\|([^{}]*?)\}\}", re.I)
LINK = re.compile(r"\[\[(?:[^|\]]*\|)?([^\]]*)\]\]")
EXTLINK = re.compile(r"\[(https?://\S+) ([^\]]*)\]")


def clean(value):
    """Turns a field value into plain text: drops refs, unwraps links and small templates."""
    value = REF.sub("", value)
    value = PATH.sub(lambda m: "{" + m.group(1) + "}", value)
    for _ in range(3):
        value = WRAP.sub(lambda m: m.group(1).split("|")[0], value)
    value = LINK.sub(r"\1", value)
    value = EXTLINK.sub(r"\2 <\1>", value)
    value = re.sub(r"<br\s*/?>", " ", value)
    value = re.sub(r"</?nowiki>|'''?", "", value)
    return re.sub(r"\s+", " ", value).strip()


def row_values(text, row):
    """Collects the positional arguments of every {{<row>|...}} in text."""
    return [split_params(body)[0] for name, body in templates(text) if name.lower() == row.lower()]


def joined(values):
    return ", ".join(dict.fromkeys(v for v in values if v))


def ids(value):
    return [v.strip() for v in clean(value).split(",") if v.strip()]


def extract(title, text):
    """Pulls the game facts out of an article, or returns None when it isn't a game."""
    record = {"title": title}
    found = {}
    for name, body in templates(text):
        found.setdefault(name.lower(), body)
    if "infobox game" not in found and "input" not in found:
        return None
    infobox = split_params(found.get("infobox game", ""))[1]

    def taxonomy(kind):
        rows = row_values(text, "Infobox game/row/taxonomy/" + kind)
        return joined(clean(v) for r in rows for v in ",".join(r).split(","))

    record["steam_appids"] = ids(infobox.get("steam appid", "")) + ids(infobox.get("steam appid side", ""))
    record["gog_ids"] = ids(infobox.get("gogcom id", "")) + ids(infobox.get("gogcom id side", ""))
    record["developers"] = joined(clean(r[0]) for r in row_values(text, "Infobox game/row/developer") if r)
    record["publishers"] = joined(clean(r[0]) for r in row_values(text, "Infobox game/row/publisher") if r)
    record["engines"] = joined(clean(r[0]) for r in row_values(text, "Infobox game/row/engine") if r)
    record["series"] = taxonomy("series")
    record["genres"] = taxonomy("genres")
    record["released"] = {clean(r[0]): clean(r[1]) for r in row_values(text, "Infobox game/row/date") if len(r) > 1}
    for key in ("official site", "wikipedia", "igdb", "lutris", "mobygames", "hltb", "winehq"):
        if infobox.get(key, "").strip():
            record[key.replace(" ", "_")] = clean(infobox[key])

    record["stores"] = [
        {"store": clean(r[0]), "id": clean(r[1]) if len(r) > 1 else "",
         "drm": clean(r[2]) if len(r) > 2 else "", "platforms": clean(r[5]) if len(r) > 5 else ""}
        for r in row_values(text, "Availability/row") if r
    ]
    for kind in ("config", "saves"):
        rows = row_values(text, "Game data/" + kind)
        record[kind + "_paths"] = [
            {"platform": clean(r[0]), "path": clean("|".join(r[1:]))} for r in rows if len(r) > 1 and r[1].strip()
        ]

    for key, template in SECTIONS.items():
        named = split_params(found.get(template.lower(), ""))[1]
        fields = {k: clean(v) for k, v in named.items()}
        record[key] = {k: v for k, v in fields.items() if v}
    linux = [record["api"].get(k, "") for k in ("linux 32-bit executable", "linux 64-bit executable")]
    record["linux_native"] = "true" if "true" in linux else ""
    return record


def tsv_row(record):
    values = dict(record)
    values["steam_appids"] = ",".join(record["steam_appids"])
    values["gog_ids"] = ",".join(record["gog_ids"])
    values["released"] = joined(f"{k}: {v}" for k, v in record["released"].items())
    for column, field in INPUT_COLUMNS.items():
        values[column] = record["input"].get(field, "")
    return "\t".join(str(values.get(c, "")).replace("\t", " ") for c in TSV_COLUMNS)


def bucket(title):
    first = title.lstrip(" .'\"!$(/").lower()[:1]
    if first.isascii() and first.isalpha():
        return first
    if first.isdigit():
        return "0-9"
    return "_"


def file_name(title, used):
    name = re.sub(r'[/\\\x00-\x1f]', "_", title).strip()
    if name.startswith("."):
        name = "_" + name[1:]
    while len(name.encode()) > 200:
        name = name[:-1]
    path = f"pages/{bucket(title)}/{name}.wiki"
    n = 2
    while path in used:
        path = f"pages/{bucket(title)}/{name} ({n}).wiki"
        n += 1
    used.add(path)
    return path


def main(dump, out):
    used = set()
    redirects = []
    games = open(os.path.join(out, "games.tsv"), "w", encoding="utf-8")
    jsonl = open(os.path.join(out, "games.jsonl"), "w", encoding="utf-8")
    appids = open(os.path.join(out, "steam-appids.tsv"), "w", encoding="utf-8")
    topics = open(os.path.join(out, "topics.tsv"), "w", encoding="utf-8")
    topics.write("title\tfile\n")
    games.write("\t".join(TSV_COLUMNS) + "\n")
    appids.write("steam_appid\ttitle\tfile\n")
    count = 0
    for _, element in ET.iterparse(dump):
        if element.tag != NS + "page":
            continue
        title = element.findtext(NS + "title")
        redirect = element.find(NS + "redirect")
        text = element.findtext(f"{NS}revision/{NS}text") or ""
        timestamp = element.findtext(f"{NS}revision/{NS}timestamp")
        element.clear()
        if redirect is not None:
            redirects.append((title, redirect.get("title")))
            continue
        path = file_name(title, used)
        os.makedirs(os.path.join(out, os.path.dirname(path)), exist_ok=True)
        url = SITE + title.replace(" ", "_")
        with open(os.path.join(out, path), "w", encoding="utf-8") as page:
            page.write(f"<!-- title: {title}\n     url: {url}\n     revised: {timestamp} -->\n{text}\n")
        record = extract(title, text)
        if record is None:
            topics.write(f"{title}\t{path}\n")
            count += 1
            continue
        record["file"] = path
        games.write(tsv_row(record) + "\n")
        jsonl.write(json.dumps({"title": title, "file": path, **record}, ensure_ascii=False) + "\n")
        for appid in record["steam_appids"]:
            appids.write(f"{appid}\t{title}\t{path}\n")
        count += 1
    with open(os.path.join(out, "redirects.tsv"), "w", encoding="utf-8") as f:
        f.write("from\tto\n")
        for source, target in sorted(redirects):
            f.write(f"{source}\t{target}\n")
    print(f"{count} pages, {len(redirects)} redirects")


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
