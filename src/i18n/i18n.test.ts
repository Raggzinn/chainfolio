import i18n, { normalizeLanguage } from "./index";

describe("normalizeLanguage", () => {
  it.each([
    ["pt", "pt-BR"],
    ["pt-PT", "pt-BR"],
    ["pt-BR", "pt-BR"],
    ["en-US", "en"],
    ["fr", "en"],
  ])("%s -> %s", (input, expected) => {
    expect(normalizeLanguage(input)).toBe(expected);
  });
});

describe("i18n", () => {
  it("switches to pt-BR translations", async () => {
    await i18n.changeLanguage("pt-BR");
    expect(i18n.t("nav.settings")).toBe("Configurações");
    await i18n.changeLanguage("en");
    expect(i18n.t("nav.settings")).toBe("Settings");
  });
});
