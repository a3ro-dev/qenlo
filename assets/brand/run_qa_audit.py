"""
Comprehensive QA Audit for Qenlo Brandkit Deliverables
"""
import os
import xml.etree.ElementTree as ET
import hashlib
from PIL import Image
from fontTools.ttLib import TTFont

def srgb_to_linear(c):
    c = c / 255.0
    return c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4

def relative_luminance(hex_str):
    hex_str = hex_str.lstrip('#')
    r, g, b = [int(hex_str[i:i+2], 16) for i in (0, 2, 4)]
    return 0.2126 * srgb_to_linear(r) + 0.7152 * srgb_to_linear(g) + 0.0722 * srgb_to_linear(b)

def contrast_ratio(hex1, hex2):
    l1 = relative_luminance(hex1)
    l2 = relative_luminance(hex2)
    lighter = max(l1, l2)
    darker = min(l1, l2)
    return (lighter + 0.05) / (darker + 0.05)

def audit_svg(path):
    tree = ET.parse(path)
    root = tree.getroot()
    vb = root.attrib.get('viewBox')
    scripts = list(root.iter('{http://www.w3.org/2000/svg}script')) + list(root.iter('script'))
    images = list(root.iter('{http://www.w3.org/2000/svg}image')) + list(root.iter('image'))
    
    # Check for text elements to ensure font independence in logo files
    texts = list(root.iter('{http://www.w3.org/2000/svg}text')) + list(root.iter('text'))
    return {
        'viewBox': vb,
        'has_scripts': len(scripts) > 0,
        'has_images': len(images) > 0,
        'text_elements_count': len(texts),
        'tag': root.tag
    }

print("=" * 60)
print("1. SVG ASSET VALIDATION")
print("=" * 60)
svgs = [
    'd:/qenloDB/assets/brand/logo/mark.svg',
    'd:/qenloDB/assets/brand/logo/mark-reversed.svg',
    'd:/qenloDB/assets/brand/logo/lockup.svg',
    'd:/qenloDB/assets/brand/logo/lockup-reversed.svg',
    'd:/qenloDB/assets/brand/logo/favicon.svg',
    'd:/qenloDB/assets/brand/social/card.svg',
]

for s in svgs:
    res = audit_svg(s)
    name = os.path.basename(s)
    print(f"[{name}] viewBox='{res['viewBox']}' | scripts={res['has_scripts']} | images={res['has_images']} | texts={res['text_elements_count']}")

print("\n" + "=" * 60)
print("2. SOCIAL CARD PNG VALIDATION")
print("=" * 60)
png_path = 'd:/qenloDB/assets/brand/social/card.png'
im = Image.open(png_path)
print(f"card.png: dimensions={im.size} (Expected 1200x630), mode={im.mode}, filesize={os.path.getsize(png_path)} bytes")

print("\n" + "=" * 60)
print("3. FONT ASSET INTEGRITY & PROVENANCE")
print("=" * 60)
fonts = [
    'IBMPlexSans-Regular.woff2',
    'IBMPlexSans-SemiBold.woff2',
    'IBMPlexMono-Regular.woff2'
]
for f in fonts:
    fp = os.path.join('d:/qenloDB/assets/brand/fonts', f)
    with open(fp, 'rb') as ff:
        data = ff.read()
        sha = hashlib.sha256(data).hexdigest()
    tt = TTFont(fp)
    upem = tt['head'].unitsPerEm
    num_glyphs = len(tt.getGlyphOrder())
    print(f"[{f}] size={len(data)} bytes | sha256={sha} | upem={upem} | glyphs={num_glyphs}")

print("\n" + "=" * 60)
print("4. WCAG 2.1 CONTRAST AUDIT")
print("=" * 60)
pairs = [
    ("Light Text on Light BG", "#1D2320", "#F7F5F0", 4.5),
    ("Light Muted on Light BG", "#5A615D", "#F7F5F0", 4.5),
    ("Light Accent on Light BG", "#B53C2F", "#F7F5F0", 4.5),
    ("Light On-Accent on Accent", "#FFFFFF", "#B53C2F", 4.5),
    ("Light Border-Strong on Canvas", "#878E8A", "#F7F5F0", 3.0),
    ("Dark Text on Dark BG", "#F7F5F0", "#171C19", 4.5),
    ("Dark Muted on Dark BG", "#AAB3AD", "#171C19", 4.5),
    ("Dark Accent on Dark BG", "#EF8B79", "#171C19", 4.5),
    ("Dark On-Accent on Accent", "#171C19", "#EF8B79", 4.5),
    ("Dark Border-Strong on Canvas", "#6E7C74", "#171C19", 3.0),
]

for name, fg, bg, target in pairs:
    ratio = contrast_ratio(fg, bg)
    status = "PASS (AAA)" if ratio >= 7.0 else ("PASS (AA)" if ratio >= target else "FAIL")
    print(f"{name:<30} {fg} / {bg} -> {ratio:5.2f}:1 [{status}] (Target: >={target}:1)")

print("=" * 60)
