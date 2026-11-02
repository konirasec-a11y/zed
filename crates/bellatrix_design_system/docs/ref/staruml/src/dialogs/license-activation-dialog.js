/*
 * Copyright (c) 2013-2014 Minkyu Lee. All rights reserved.
 *
 * NOTICE:  All information contained herein is, and remains the
 * property of Minkyu Lee. The intellectual and technical concepts
 * contained herein are proprietary to Minkyu Lee and may be covered
 * by Republic of Korea and Foreign Patents, patents in process,
 * and are protected by trade secret or copyright law.
 * Dissemination of this information or reproduction of this material
 * is strictly forbidden unless prior written permission is obtained
 * from Minkyu Lee (niklaus.lee@gmail.com).
 *
 */

const { shell, clipboard } = require("electron");
const fs = require("fs");
const path = require("path");
const Mustache = require("mustache");
const Strings = require("../strings");

const licenseActivationDialogTemplate = fs.readFileSync(
  path.join(
    __dirname,
    "../static/html-contents/license-activation-dialog.html",
  ),
  "utf8",
);

async function updateDialog($dlg) {
  await app.licenseStore.fetch();
  const licenseStatus = app.licenseStore.getLicenseStatus();

  const $sectionTrialNotExpired = $dlg.find(".trial-not-expired");
  const $sectionTrialExpired = $dlg.find(".trial-expired");
  const $sectionLicenseActivated = $dlg.find(".license-activated");
  const $sectionLicenseNotActivated = $dlg.find(".license-not-activated");
  const $trialDaysLeft = $dlg.find(".trial-days-left");
  const $activeStatus = $dlg.find(".active-status");
  const $productDisplayName = $dlg.find(".product-display-name");
  const $licenseHolderName = $dlg.find(".license-holder-name");
  const $deviceId = $dlg.find(".device-id");

  $sectionTrialNotExpired.hide();
  $sectionTrialExpired.hide();
  $sectionLicenseActivated.hide();
  $sectionLicenseNotActivated.hide();

  if (licenseStatus.trial && licenseStatus.trialDaysLeft > 0) {
    $sectionTrialNotExpired.show();
  }
  if (licenseStatus.trial && licenseStatus.trialDaysLeft <= 0) {
    $sectionTrialExpired.show();
  }
  if (licenseStatus.activated) {
    $sectionLicenseActivated.show();
  }
  if (!licenseStatus.activated) {
    $sectionLicenseNotActivated.show();
  }

  $trialDaysLeft.text(licenseStatus.trialDaysLeft.toString());
  $activeStatus.text(
    (licenseStatus.activated ? "Activated" : "Not Activated") +
      (licenseStatus.deviceId === "*" ? " (offline)" : ""),
  );
  $productDisplayName.text(licenseStatus.productDisplayName || "N/A");
  $licenseHolderName.text(licenseStatus.name || "N/A");
  if (licenseStatus.deviceId) {
    $deviceId.text(
      licenseStatus.deviceId === "*"
        ? "*"
        : licenseStatus.deviceId.slice(0, 10) + "‥",
    );
  }
}

/**
 * Show License Activation Dialog
 * @private
 * @return {Dialog}
 */
async function showDialog() {
  await app.licenseStore.fetch();

  const context = {
    Strings: Strings,
    metadata: global.app.metadata,
  };
  const dialog = app.dialogs.showModalDialogUsingTemplate(
    Mustache.render(licenseActivationDialogTemplate, context),
  );

  const $dlg = dialog.getElement();
  const $buyButton = $dlg.find(".buy-button");
  const $licenseKey = $dlg.find(".license-key");
  const $activateButton = $dlg.find(".activate-button");
  const $deactivateButton = $dlg.find(".deactivate-button");
  const $copyDeviceIdButton = $dlg.find(".copy-device-id-button");
  const $licenseManagerButton = $dlg.find(".license-manager-button");
  await updateDialog($dlg);

  $activateButton.click(async function () {
    const licenseKey = $licenseKey.val().trim();
    try {
      await app.licenseStore.activate(licenseKey);
      setTimeout(async () => {
        await updateDialog($dlg);
      }, 0);
    } catch (err) {
      console.error("Failed to activate license:", err);
      app.dialogs.showAlertDialog("Failed to activate license.");
    }
  });

  $deactivateButton.click(async function () {
    try {
      await app.licenseStore.deactivate();
      setTimeout(async () => {
        await updateDialog($dlg);
      }, 0);
    } catch (err) {
      console.error("Failed to deactivate license:", err);
      app.dialogs.showAlertDialog("Failed to deactivate license.");
    }
  });

  $buyButton.click(function () {
    shell.openExternal(app.config.purchase_url);
  });

  $copyDeviceIdButton.click(async function () {
    const deviceId = await app.licenseStore.getDeviceId();
    if (deviceId) {
      clipboard.writeText(deviceId);
      app.toast.info("Device ID copied to clipboard.");
    } else {
      app.toast.error("Failed to get Device ID.");
    }
  });

  $licenseManagerButton.click(function () {
    shell.openExternal(app.config.license_manager_url);
  });

  dialog.then(function ({ buttonId }) {
    const licenseStatus = app.licenseStore.getLicenseStatus();
    if (
      buttonId === "cancel" &&
      licenseStatus.trial &&
      licenseStatus.trialDaysLeft <= 0
    ) {
      // viewer mode (readonly mode)
      app.repository._readonly = true;
      app.diagrams.diagramEditor._readonly = true;
      app.propertyEditorView._readonly = true;
    }
  });

  return dialog;
}

exports.showDialog = showDialog;
