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
    const totals = page.locator(".totals");
    await expect(totals.getByText("37.50 EGP")).toBeVisible();
  });

  test("يعطي حقل البند وقائمة الاختيارات مساحة كافية", async ({ page }) => {
    const input = page.getByPlaceholder("ابحث عن صنف أو اكتب اسمًا جديدًا").first();

    // The item field must never be squeezed into a stub.
    const field = await input.boundingBox();
    expect(field?.width ?? 0).toBeGreaterThanOrEqual(240);

    // The suggestion list must be readable even when the field is narrow.
    await input.click();
    await input.fill("ح");
    const list = page.locator(".combobox__list").first();
    await expect(list).toBeVisible();
    const options = await list.boundingBox();
    expect(options?.width ?? 0).toBeGreaterThanOrEqual(400);
    await expect(list.getByText("حليب كامل الدسم")).toBeVisible();
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

  test("يبقي التركيز في حقل الاسم أثناء كتابة صنف جديد", async ({ page }) => {
    await page.getByRole("link", { name: "الأصناف" }).click();
    await page.getByRole("button", { name: "صنف جديد" }).click();

    const name = page.getByLabel("اسم الصنف");
    await name.click();
    await name.pressSequentially("حليب كامل الدسم");

    // The caret must stay in the field while the user types.
    await expect(name).toBeFocused();
    await expect(name).toHaveValue("حليب كامل الدسم");
  });

  test("يفتح معاينة الفاتورة في نافذة منبثقة", async ({ page }) => {
    await page.getByLabel("اسم العميل").fill("عميل تجريبي");
    await fillLine(page, "منتج تجريبي", "2", "50.00");

    await page.getByRole("button", { name: "معاينة الفاتورة" }).click();

    const dialog = page.getByRole("dialog", { name: "معاينة الفاتورة" });
    await expect(dialog).toBeVisible();

    // Scope to the rendered page: the document also renders a hidden copy of
    // the table that is used to measure the pagination.
    const invoicePage = dialog.locator(".invoice-page").first();
    await expect(invoicePage.getByText("منتج تجريبي")).toBeVisible();
    await expect(invoicePage.getByText("100.00").first()).toBeVisible();

    await dialog.locator(".modal__footer").getByRole("button", { name: "إغلاق" }).click();
    await expect(dialog).toHaveCount(0);
  });

  test("يختار بنود الفاتورة ويطبعها في آخر الفاتورة", async ({ page }) => {
    await page.getByLabel("اسم العميل").fill("عميل تجريبي");
    await fillLine(page, "منتج تجريبي", "2", "50.00");

    const terms = page.locator(".term-picker__item");
    await expect(terms).toHaveCount(2);
    await terms.filter({ hasText: "حقوق النشر محفوظة" }).click();
    await terms.filter({ hasText: "ضمان شركة الأبنودي" }).click();

    await page.getByRole("button", { name: "معاينة الفاتورة" }).click();
    const invoicePage = page
      .getByRole("dialog", { name: "معاينة الفاتورة" })
      .locator(".invoice-page")
      .first();
    await expect(invoicePage.locator(".invoice-terms__item")).toHaveCount(2);
    await expect(invoicePage.getByText("حقوق النشر محفوظة")).toBeVisible();
    await expect(invoicePage.getByText("ضمان شركة الأبنودي لمدة ثلاث أعوام")).toBeVisible();

    // The terms survive the issue and stay on the stored invoice.
    await page
      .getByRole("dialog", { name: "معاينة الفاتورة" })
      .locator(".modal__footer")
      .getByRole("button", { name: "إغلاق" })
      .click();
    await page.getByRole("button", { name: "إصدار الفاتورة" }).click();
    await expect(page.locator(".invoice-page .invoice-terms__item")).toHaveCount(2);
  });

  test("يخفي اسم الشركة عندما لا يكون مكتوبًا", async ({ page }) => {
    // A company name that was never filled in — and the placeholder older
    // installs stored — never shows up. The app name is not used as a
    // stand-in either: the sidebar shows the logo and nothing else.
    await expect(page.locator(".sidebar__brand-name")).toHaveCount(0);
    await expect(page.locator(".sidebar__brand-sub")).toHaveCount(0);
    await expect(page.getByText("اسم الشركة")).toHaveCount(0);
    await expect(page.locator(".topbar__meta")).toHaveText("EGP");
  });

  test("يعرض اسم الشركة في الشريط الجانبي بعد كتابته", async ({ page }) => {
    await page.getByRole("link", { name: "الإعدادات" }).click();
    await page.getByPlaceholder("اسم الشركة").fill("شركة الأبنودي");
    await page.getByRole("button", { name: "حفظ الإعدادات" }).click();

    await expect(page.locator(".sidebar__brand-name")).toHaveText("شركة الأبنودي");
    await expect(page.locator(".sidebar__brand-sub")).toHaveText("برنامج الفواتير");
    await expect(page.locator(".topbar__meta")).toHaveText("EGP — شركة الأبنودي");
  });

  test("يغيّر لون الشريط العلوي مع لون العلامة", async ({ page }) => {
    const topbar = page.locator(".topbar");
    const before = await topbar.evaluate((el) => getComputedStyle(el).backgroundColor);

    await page.getByRole("link", { name: "الإعدادات" }).click();
    await page.getByLabel("اختيار اللون").fill("#c0392b");

    // The header follows the picked colour straight away.
    await expect
      .poll(async () => topbar.evaluate((el) => getComputedStyle(el).backgroundColor))
      .not.toBe(before);
    await expect
      .poll(async () => topbar.evaluate((el) => getComputedStyle(el).backgroundColor))
      .toBe("rgb(192, 57, 43)");

    // Saving keeps it.
    await page.getByRole("button", { name: "حفظ الإعدادات" }).click();
    await expect(page.getByText("تم حفظ الإعدادات")).toBeVisible();
    await expect
      .poll(async () => topbar.evaluate((el) => getComputedStyle(el).backgroundColor))
      .toBe("rgb(192, 57, 43)");
  });

  test("يستخدم خط Cairo في الواجهة", async ({ page }) => {
    const font = await page.locator("body").evaluate((el) => getComputedStyle(el).fontFamily);
    expect(font).toContain("Cairo");

    const loaded = await page.evaluate(async () => {
      await document.fonts.ready;
      return document.fonts.check("14px Cairo");
    });
    expect(loaded).toBe(true);
  });

  test("يطبّق خصم النسبة ويطبع تفصيل الخصم في الفاتورة", async ({ page }) => {
    await page.getByLabel("اسم العميل").fill("عميل تجريبي");
    await fillLine(page, "منتج تجريبي", "2", "50.00");

    await page.locator("#discount-kind").selectOption("percent");
    await page.getByLabel("قيمة الخصم").fill("10");

    // 2 × 50.00 = 100.00, minus 10% = 90.00.
    const totals = page.locator(".totals");
    await expect(totals.getByText("100.00 EGP")).toBeVisible();
    await expect(totals.getByText("10.00 EGP")).toBeVisible();
    await expect(totals.getByText("90.00 EGP")).toBeVisible();

    // The document shows the breakdown: subtotal, discount, net total.
    await page.getByRole("button", { name: "معاينة الفاتورة" }).click();
    const invoicePage = page
      .getByRole("dialog", { name: "معاينة الفاتورة" })
      .locator(".invoice-page")
      .first();
    const box = invoicePage.locator(".invoice-totals__box");
    await expect(box.locator(".invoice-totals__row--subtotal")).toContainText("100.00");
    await expect(box.locator(".invoice-totals__row--discount")).toContainText("10.00");
    await expect(box.locator(".invoice-totals__row--discount")).toContainText("10%");
    await expect(box.locator(".invoice-totals__row--grand")).toContainText("90.00");
  });

  test("يرفض نسبة خصم أكبر من مئة بالمئة", async ({ page }) => {
    await page.getByLabel("اسم العميل").fill("عميل تجريبي");
    await fillLine(page, "منتج تجريبي", "2", "50.00");

    await page.locator("#discount-kind").selectOption("percent");
    await page.getByLabel("قيمة الخصم").fill("120");
    await expect(page.locator(".totals__hint")).toHaveText("نسبة الخصم لا يمكن أن تتجاوز 100%");

    // The invoice cannot be saved with that discount.
    await page.getByRole("button", { name: "إصدار الفاتورة" }).click();
    await expect(page.getByText("نسبة الخصم لا يمكن أن تتجاوز 100%").first()).toBeVisible();
  });

  test("يطبّق خصم المبلغ ويحفظه مع الفاتورة", async ({ page }) => {
    await page.getByLabel("اسم العميل").fill("عميل تجريبي");
    await fillLine(page, "منتج تجريبي", "2", "50.00");

    await page.locator("#discount-kind").selectOption("amount");
    await page.getByLabel("قيمة الخصم").fill("25.00");

    const totals = page.locator(".totals");
    await expect(totals.getByText("25.00 EGP")).toBeVisible();
    await expect(totals.getByText("75.00 EGP")).toBeVisible();

    // The discount travels with the issued invoice.
    await page.getByRole("button", { name: "إصدار الفاتورة" }).click();
    const box = page.locator(".invoice-page .invoice-totals__box").first();
    await expect(box.locator(".invoice-totals__row--discount")).toContainText("25.00");
    await expect(box.locator(".invoice-totals__row--grand")).toContainText("75.00");
  });
});
