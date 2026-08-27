"""
Render screenshots of preview.html to inspect desktop and mobile responsive layouts.
"""
import os
from playwright.sync_api import sync_playwright

html_path = 'd:/qenloDB/assets/brand/preview.html'
desktop_png = 'd:/qenloDB/assets/brand/preview_desktop.png'
mobile_png = 'd:/qenloDB/assets/brand/preview_mobile.png'

with sync_playwright() as p:
    browser = p.chromium.launch()
    
    # Desktop
    page = browser.new_page(viewport={'width': 1280, 'height': 2400})
    page.goto(f'file:///{os.path.abspath(html_path).replace(os.sep, "/")}')
    page.screenshot(path=desktop_png, full_page=True)
    
    # Mobile
    page_mobile = browser.new_page(viewport={'width': 375, 'height': 2400})
    page_mobile.goto(f'file:///{os.path.abspath(html_path).replace(os.sep, "/")}')
    page_mobile.screenshot(path=mobile_png, full_page=True)
    
    browser.close()

print('Screenshots rendered successfully.')
