import { usePreferences } from "./preferences";

describe("preferences store", () => {
  beforeEach(() => {
    usePreferences.setState({ currency: "USD", privacyMode: false, taxModuleEnabled: false });
  });

  it("defaults the tax module to disabled", () => {
    expect(usePreferences.getState().taxModuleEnabled).toBe(false);
  });

  it("toggles privacy mode", () => {
    usePreferences.getState().togglePrivacyMode();
    expect(usePreferences.getState().privacyMode).toBe(true);
    usePreferences.getState().togglePrivacyMode();
    expect(usePreferences.getState().privacyMode).toBe(false);
  });

  it("changes the base currency", () => {
    usePreferences.getState().setCurrency("BRL");
    expect(usePreferences.getState().currency).toBe("BRL");
  });
});
