import { expect, test } from "@playwright/test";

/**
 * The end-to-end suite drives the real UI in a browser.
 *
 * These checks cover the behaviour that does not depend on Windows or on the
 * Rust backend: the RTL layout, the product combobox, the per-line price
 * override, the live total and the invoice lifecycle.
 */

/** Fill the first invoice line with a name, a quantity and a unit price. */
async function fillLine(
  page: import("@playwright/test").Page,
  name: string,
  quantity: string,
  price: string,
) {
  await page.getByPlaceholder("ابحث عن صنف أو اكتب اسمًا جديدًا").first().fill(name);
  await page.getByLabel("الكمية").first().fill(quantity);
  await page.getByLabel("سعر الوحدة").first().fill(price);
}

test.describe("برنامج الفواتير", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    await expect(page.getByRole("heading", { name: "فاتورة جديدة" })).toBeVisible();
  });

  test("يعرض واجهة عربية من اليمين إلى اليسار", async ({ page }) => {
    await expect(page.locator("html")).toHaveAttribute("dir", "rtl");
    await expect(page.locator("html")).toHaveAttribute("lang", "ar");
    await expect(page.getByRole("navigation")).toBeVisible();
  });

  test("يحسب الإجمالي من الكمية وسعر الوحدة", async ({ page }) => {
    await page.getByLabel("اسم العميل").fill("عميل تجريبي");
    await fillLine(page, "منتج تجريبي", "3", "12.50");

    // 3 × 12.50 = 37.50
    await expect(page.getByText("37.50").first()).toBeVisible();
    const summary = page.locator(".card").filter({ hasText: "عدد البنود" });
    await expect(summary.getByText("37.50 EGP")).toBeVisible();
  });

  test("يسمح بتغيير سعر السطر دون تغيير سعر الصنف في القائمة", async ({ page }) => {
    await page.getByLabel("اسم العميل").fill("عميل تجريبي");
    await fillLine(page, "منتج تجريبي", "1", "99.00");
    await expect(page.getByText("99.00").first()).toBeVisible();

    // The catalog still holds the original price.
    await page.getByRole("link", { name: "الأصناف" }).click();
    await expect(page.getByRole("heading", { name: "الأصناف" })).toBeVisible();
    await expect(page.getByText("25.00").first()).toBeVisible();
  });

  test("يبحث في الأصناف فورًا", async ({ page }) => {
    await page.getByRole("link", { name: "الأصناف" }).click();
    await page.getByLabel("بحث الأصناف").fill("زيت");
    await expect(page.getByRole("cell", { name: "زيت زيتون بكر" })).toBeVisible();
    await expect(page.getByRole("cell", { name: "حليب كامل الدسم" })).toHaveCount(0);
  });

  test("يصدر فاتورة ويظهرها في السجل", async ({ page }) => {
    await page.getByLabel("اسم العميل").fill("عميل تجريبي");
    await fillLine(page, "منتج تجريبي", "2", "50.00");

    await page.getByRole("button", { name: "إصدار الفاتورة" }).click();

    await expect(page.getByRole("heading", { name: /INV-000001/ })).toBeVisible();
    await expect(page.getByText("عميل تجريبي").first()).toBeVisible();

    await page.getByRole("button", { name: "رجوع" }).click();
    await page.getByRole("link", { name: "سجل الفواتير" }).click();
    await expect(page.getByRole("cell", { name: "INV-000001" })).toBeVisible();
  });

  test("يحفظ مسودة غير محفوظة ويستعيدها", async ({ page }) => {
    await page.getByLabel("اسم العميل").fill("مسودة العميل");
    await page.waitForTimeout(1500);

    await page.reload();
    await expect(page.getByText("يوجد مسودة غير محفوظة من جلسة سابقة")).toBeVisible();
    await page.getByRole("button", { name: "استعادة المسودة" }).click();
    await expect(page.getByLabel("اسم العميل")).toHaveValue("مسودة العميل");
  });
});
