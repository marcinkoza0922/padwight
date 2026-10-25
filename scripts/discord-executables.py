#!/usr/bin/env python3
"""Flatten Discord's detectable-applications list into executables.tsv beside the PCGW snapshot.

Usage: python3 -I scripts/discord-executables.py pcgamingwiki/discord/detectable.json pcgamingwiki

Fetch the list first:
  curl -sSf -A 'Mozilla/5.0' -o pcgamingwiki/discord/detectable.json \
    https://discord.com/api/applications/detectable

Writes <out>/executables.tsv, one row per executable, joined to games.tsv by Steam app id:
  executable  os  launcher  arguments  discord_name  steam_appids  other_skus  pcgw_title  pcgw_file
"""

import csv
import json
import os
import sys


def main(detectable, out):
    pcgw = {}
    with open(os.path.join(out, "steam-appids.tsv"), encoding="utf-8") as f:
        for row in csv.DictReader(f, delimiter="\t", quoting=csv.QUOTE_NONE):
            pcgw.setdefault(row["steam_appid"], (row["title"], row["file"]))
    with open(detectable, encoding="utf-8") as f:
        apps = json.load(f)
    rows = 0
    with open(os.path.join(out, "executables.tsv"), "w", encoding="utf-8") as f:
        f.write("executable\tos\tlauncher\targuments\tdiscord_name\tsteam_appids\tother_skus\tpcgw_title\tpcgw_file\n")
        for app in apps:
            steam = [s["id"] for s in app["third_party_skus"] if s["distributor"] == "steam" and s["id"]]
            other = [f'{s["distributor"]}:{s["id"]}' for s in app["third_party_skus"] if s["distributor"] != "steam"]
            title, path = next((pcgw[a] for a in steam if a in pcgw), ("", ""))
            for exe in app["executables"]:
                fields = [
                    exe["name"], exe["os"], "launcher" if exe["is_launcher"] else "",
                    exe.get("arguments", ""), app["name"], ",".join(steam), ",".join(other), title, path,
                ]
                f.write("\t".join(str(v).replace("\t", " ") for v in fields) + "\n")
                rows += 1
    print(f"{rows} executables from {len(apps)} apps")


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
