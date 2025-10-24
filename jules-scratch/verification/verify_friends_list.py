
import asyncio
from playwright.async_api import async_playwright, expect

async def main():
    async with async_playwright() as p:
        browser = await p.chromium.launch()
        page = await browser.new_page()
        await page.goto("file:///app/matchbox_web/dist/test.html")

        # Give it a moment to load scripts
        await page.wait_for_timeout(1000)

        # Print the content to debug
        content = await page.content()
        print(content)

        # Wait for the component to be visible
        friends_list_component = page.locator("matchbox-friends-list")
        await expect(friends_list_component).to_be_visible()

        await page.screenshot(path="jules-scratch/verification/verification.png")
        await browser.close()

asyncio.run(main())
