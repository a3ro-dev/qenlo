#!/usr/bin/env python3
"""
Qenlo Brandkit - Font Downloader & Token Parity Validator
Downloads official IBM Plex font assets (@ibm/plex-sans@1.1.0, @ibm/plex-mono@2.5.0),
validates font binary structures via fontTools, records SHA256 hashes,
and verifies 100% parity between tokens.json and tokens.css.
"""

import os
import sys
import json
import hashlib
import urllib.request
import re

FONTS_DIR = os.path.join(os.path.dirname(__file__), "fonts")
TOKENS_JSON_PATH = os.path.join(os.path.dirname(__file__), "tokens.json")
TOKENS_CSS_PATH = os.path.join(os.path.dirname(__file__), "tokens.css")
OFL_PATH = os.path.join(FONTS_DIR, "OFL.txt")

FONT_SOURCES = {
    "IBMPlexSans-Regular.woff2": {
        "package": "@ibm/plex-sans@1.1.0",
        "release_tag": "@ibm/plex-sans@1.1.0",
        "upstream_urls": [
            "https://raw.githubusercontent.com/IBM/plex/master/packages/plex-sans/fonts/complete/woff2/IBMPlexSans-Regular.woff2",
            "https://cdn.jsdelivr.net/npm/@ibm/plex-sans@1.1.0/fonts/complete/woff2/IBMPlexSans-Regular.woff2",
            "https://unpkg.com/@ibm/plex-sans@1.1.0/fonts/complete/woff2/IBMPlexSans-Regular.woff2"
        ]
    },
    "IBMPlexSans-SemiBold.woff2": {
        "package": "@ibm/plex-sans@1.1.0",
        "release_tag": "@ibm/plex-sans@1.1.0",
        "upstream_urls": [
            "https://raw.githubusercontent.com/IBM/plex/master/packages/plex-sans/fonts/complete/woff2/IBMPlexSans-SemiBold.woff2",
            "https://cdn.jsdelivr.net/npm/@ibm/plex-sans@1.1.0/fonts/complete/woff2/IBMPlexSans-SemiBold.woff2",
            "https://unpkg.com/@ibm/plex-sans@1.1.0/fonts/complete/woff2/IBMPlexSans-SemiBold.woff2"
        ]
    },
    "IBMPlexMono-Regular.woff2": {
        "package": "@ibm/plex-mono@2.5.0",
        "release_tag": "@ibm/plex-mono@2.5.0",
        "upstream_urls": [
            "https://raw.githubusercontent.com/IBM/plex/master/packages/plex-mono/fonts/complete/woff2/IBMPlexMono-Regular.woff2",
            "https://cdn.jsdelivr.net/npm/@ibm/plex-mono@2.5.0/fonts/complete/woff2/IBMPlexMono-Regular.woff2",
            "https://unpkg.com/@ibm/plex-mono@2.5.0/fonts/complete/woff2/IBMPlexMono-Regular.woff2"
        ]
    }
}

def compute_sha256(filepath):
    h = hashlib.sha256()
    with open(filepath, "rb") as f:
        while chunk := f.read(8192):
            h.update(chunk)
    return h.hexdigest()

def download_fonts():
    os.makedirs(FONTS_DIR, exist_ok=True)
    results = {}
    
    for filename, meta in FONT_SOURCES.items():
        target_path = os.path.join(FONTS_DIR, filename)
        downloaded = False
        last_err = None
        
        # Check if already present and valid
        if os.path.exists(target_path) and os.path.getsize(target_path) > 1000:
            print(f"[FOUND] {filename} exists ({os.path.getsize(target_path)} bytes)")
            downloaded = True
        else:
            for url in meta["upstream_urls"]:
                try:
                    print(f"[DOWNLOADING] {filename} from {url}...")
                    req = urllib.request.Request(
                        url,
                        headers={"User-Agent": "Qenlo-Brandkit-Installer/1.0"}
                    )
                    with urllib.request.urlopen(req, timeout=15) as resp:
                        content = resp.read()
                        if len(content) > 1000:
                            with open(target_path, "wb") as f:
                                f.write(content)
                            print(f"[SUCCESS] Downloaded {filename} ({len(content)} bytes)")
                            downloaded = True
                            break
                except Exception as e:
                    last_err = e
                    continue
        
        if not downloaded:
            print(f"[WARN] Could not download {filename}: {last_err}")
        else:
            sha = compute_sha256(target_path)
            results[filename] = {
                "path": target_path,
                "size_bytes": os.path.getsize(target_path),
                "sha256": sha,
                "release_tag": meta["release_tag"],
                "upstream_url": meta["upstream_urls"][0]
            }
            
    return results

def validate_fonts():
    report = []
    try:
        from fontTools.ttLib import TTFont
        has_fonttools = True
    except ImportError:
        has_fonttools = False
        print("[INFO] fontTools package not installed in active environment. Performing header & size validation.")

    for filename in FONT_SOURCES.keys():
        target_path = os.path.join(FONTS_DIR, filename)
        if not os.path.exists(target_path):
            report.append({"file": filename, "status": "MISSING"})
            continue
            
        size = os.path.getsize(target_path)
        sha = compute_sha256(target_path)
        
        # Basic woff2 signature check ('wOF2' magic number: 0x774F4632)
        with open(target_path, "rb") as f:
            magic = f.read(4)
        is_woff2 = (magic == b"wOF2")
        
        details = {
            "file": filename,
            "path": target_path,
            "size_bytes": size,
            "sha256": sha,
            "is_valid_woff2_header": is_woff2
        }
        
        if has_fonttools and is_woff2:
            try:
                font = TTFont(target_path)
                tables = list(font.keys())
                num_glyphs = font['maxp'].numGlyphs if 'maxp' in font else 'N/A'
                details["tables"] = tables
                details["num_glyphs"] = num_glyphs
                details["status"] = "VALID"
            except Exception as e:
                details["status"] = f"INVALID: {e}"
        else:
            details["status"] = "VALID_HEADER" if is_woff2 else "INVALID_MAGIC"
            
        report.append(details)
    return report

def verify_token_parity():
    with open(TOKENS_JSON_PATH, "r", encoding="utf-8") as f:
        tokens_json = json.load(f)
    
    with open(TOKENS_CSS_PATH, "r", encoding="utf-8") as f:
        tokens_css = f.read()

    qenlo_tokens = tokens_json.get("qenlo", {})
    mismatches = []
    matched = 0

    # 1. Colors
    light_colors = qenlo_tokens.get("color", {}).get("light", {})
    for key, val_obj in light_colors.items():
        var_name = f"--qenlo-color-{key}"
        expected_val = val_obj["$value"]
        if f"{var_name}: {expected_val};" not in tokens_css:
            mismatches.append(f"Light color mismatch: {var_name} should be {expected_val}")
        else:
            matched += 1

    dark_colors = qenlo_tokens.get("color", {}).get("dark", {})
    for key, val_obj in dark_colors.items():
        var_name = f"--qenlo-color-{key}"
        expected_val = val_obj["$value"]
        if f"{var_name}: {expected_val};" not in tokens_css:
            mismatches.append(f"Dark color mismatch: {var_name} should be {expected_val}")
        else:
            matched += 1

    # 2. Font Families
    font_families = qenlo_tokens.get("font-family", {})
    for key, val_obj in font_families.items():
        var_name = f"--qenlo-font-family-{key}"
        expected_val = val_obj["$value"]
        if f"{var_name}: {expected_val};" not in tokens_css:
            mismatches.append(f"Font family mismatch: {var_name} should be {expected_val}")
        else:
            matched += 1

    # 3. Font Weights
    font_weights = qenlo_tokens.get("font-weight", {})
    for key, val_obj in font_weights.items():
        var_name = f"--qenlo-font-weight-{key}"
        expected_val = str(val_obj["$value"])
        if f"{var_name}: {expected_val};" not in tokens_css:
            mismatches.append(f"Font weight mismatch: {var_name} should be {expected_val}")
        else:
            matched += 1

    # 4. Font Sizes
    font_sizes = qenlo_tokens.get("font-size", {})
    for key, val_obj in font_sizes.items():
        var_name = f"--qenlo-font-size-{key}"
        expected_val = val_obj["$value"]
        if f"{var_name}: {expected_val};" not in tokens_css:
            mismatches.append(f"Font size mismatch: {var_name} should be {expected_val}")
        else:
            matched += 1

    # 5. Space
    spacing = qenlo_tokens.get("space", {})
    for key, val_obj in spacing.items():
        var_name = f"--qenlo-space-{key}"
        expected_val = val_obj["$value"]
        if f"{var_name}: {expected_val};" not in tokens_css:
            mismatches.append(f"Space mismatch: {var_name} should be {expected_val}")
        else:
            matched += 1

    return {
        "matched_tokens_count": matched,
        "mismatches": mismatches,
        "parity_100_percent": len(mismatches) == 0
    }

if __name__ == "__main__":
    print("=" * 60)
    print("1. Downloading fonts...")
    dl_results = download_fonts()
    
    print("\n2. Validating font binaries...")
    val_report = validate_fonts()
    for v in val_report:
        print(f" - {v.get('file')}: Status={v.get('status')}, SHA256={v.get('sha256', 'N/A')}")
        
    print("\n3. Verifying Token Parity...")
    parity = verify_token_parity()
    print(f" - Matched Tokens: {parity['matched_tokens_count']}")
    print(f" - Mismatches: {len(parity['mismatches'])}")
    print(f" - Parity: {'100% OK' if parity['parity_100_percent'] else 'FAILED'}")
    print("=" * 60)
