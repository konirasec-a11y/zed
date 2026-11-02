/* global globalThis */

const { app } = require("electron");
const { machineId } = require("node-machine-id");
const path = require("path");
const fs = require("node:fs/promises");
const packageJson = require("../../package.json");

const crypto = globalThis.crypto;

const LICENSE_PRODUCT_ID = packageJson.productId;
const LICENSE_CRYPTO_KEY = "y0JMc9mvB1uvIi82GhdMJQXzVJxl+1Lc0RqZqWaQvx0=";
const LICENSE_SERVER_URL = "https://staruml.io/api/license-manager";
const LICENSE_FILE_NAME = "activation.key";
const LICENSE_TRIAL_MODE_ENABLED = true;
const LICENSE_TRIAL_TIMESTAMP_FILE_NAME = "lib.so";
const LICENSE_TRIAL_MODE_DAYS = 30; // negative for infinite, positive for days left

const PRODUCT_DISPLAY_NAME = {
  "STARUML.V2": "StarUML V2",
  "STARUML.V3": "StarUML V3",
  "STARUML.V4": "StarUML V4",
  "STARUML.V5": "StarUML V5",
  "STARUML.V6": "StarUML V6",
  "STARUML.V7": "StarUML V7",
};

const EDITION_DISPLAY_NAME = {
  STD: "Standard",
  PRO: "Professional",
  CO: "Commercial",
  ED: "Educational",
  PS: "Personal",
  CR: "Classroom",
  CAMPUS: "Campus",
  SITE: "Site",
};

let licenseStatus = {
  activated: false,
  name: null,
  product: null,
  edition: null,
  productDisplayName: null,
  deviceId: null,
  licenseKey: null,
  activationCode: null,
  trial: false,
  trialDaysLeft: 0,
};

/**
 * Base64 decoding (Base64 string -> Uint8Array)
 */
function base64ToArrayBuffer(base64) {
  const binary = atob(base64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i);
  }
  return bytes;
}

/**
 * Import AES key from Base64 string
 */
async function importAESKey(base64Key) {
  const keyBuffer = base64ToArrayBuffer(base64Key);
  const imported = await crypto.subtle.importKey(
    "raw",
    keyBuffer,
    { name: "AES-GCM" },
    true,
    ["encrypt", "decrypt"],
  );
  return imported;
}

/**
 * Decryption function
 */
async function decryptString(encryptedText, key) {
  const [ivBase64, encryptedBase64] = encryptedText.split(":");
  const iv = base64ToArrayBuffer(ivBase64);
  const encryptedBuffer = base64ToArrayBuffer(encryptedBase64);
  const decrypted = await crypto.subtle.decrypt(
    {
      name: "AES-GCM",
      iv,
    },
    key,
    encryptedBuffer,
  );
  return new TextDecoder().decode(decrypted);
}

/**
 * Decode an activation code
 * @returns {object} - The decoded activation code
 */
async function decodeActivationCode(activationCode) {
  const key = await importAESKey(LICENSE_CRYPTO_KEY);
  const decrypted = await decryptString(activationCode, key);
  return JSON.parse(decrypted);
}

/**
 * Decode the validation code to get the device ID.
 * @returns {object} - The decoded device ID
 */
async function decodeValidationCode(validationCode) {
  const key = await importAESKey(LICENSE_CRYPTO_KEY);
  const decrypted = await decryptString(validationCode, key);
  const deviceId = decrypted;
  return { deviceId };
}

async function getDeviceId() {
  const id = await machineId();
  return id;
}

async function checkTrialMode() {
  if (LICENSE_TRIAL_MODE_ENABLED) {
    licenseStatus.trial = true;
    if (LICENSE_TRIAL_MODE_DAYS < 0) {
      licenseStatus = {
        ...licenseStatus,
        trialDaysLeft: -1, // infinite (no time limit)
      };
      // eslint-disable-next-line no-useless-return
      return;
    } else {
      const filePath = path.join(
        app.getPath("userData"),
        LICENSE_TRIAL_TIMESTAMP_FILE_NAME,
      );
      try {
        const timestamp = await fs.readFile(filePath, "utf8");
        const now = Date.now();
        const trialStart = parseInt(timestamp, 10);
        const trialEnd =
          trialStart + LICENSE_TRIAL_MODE_DAYS * 24 * 60 * 60 * 1000;
        if (now > trialEnd) {
          licenseStatus = {
            ...licenseStatus,
            trialDaysLeft: 0,
          };
        } else {
          licenseStatus = {
            ...licenseStatus,
            trialDaysLeft: Math.floor((trialEnd - now) / (24 * 60 * 60 * 1000)),
          };
        }
      } catch (err) {
        // if the file does not exist, create it with the current timestamp
        const now = Date.now();
        await fs.writeFile(filePath, now.toString(), "utf8");
        licenseStatus = {
          ...licenseStatus,
          trial: true,
          trialDaysLeft: LICENSE_TRIAL_MODE_DAYS,
        };
      }
    }
  }
}

async function localValidate() {
  try {
    const filePath = path.join(app.getPath("userData"), LICENSE_FILE_NAME);
    const activationCode = await fs.readFile(filePath, "utf8");
    const decoded = await decodeActivationCode(activationCode);
    const deviceId = await getDeviceId();
    const isProductMatched = decoded.product === LICENSE_PRODUCT_ID;
    const isDeviceIdMatched =
      decoded.deviceId === "*" || decoded.deviceId === deviceId;

    if (!isProductMatched) {
      await checkTrialMode();
      await localDeactivate();
      return {
        success: false,
        message: "Invalid activation code (product mismatch)",
      };
    }

    if (!isDeviceIdMatched) {
      await checkTrialMode();
      await localDeactivate();
      return {
        success: false,
        message: "Invalid activation code (device ID mismatch)",
      };
    }

    licenseStatus = {
      activated: true,
      name: decoded.name,
      product: decoded.product,
      edition: decoded.edition,
      productDisplayName: getProductDisplayName(
        decoded.product,
        decoded.edition,
      ),
      deviceId: decoded.deviceId,
      licenseKey: decoded.licenseKey,
      activationCode: activationCode,
      trial: false,
      trialDaysLeft: 0,
    };
    return {
      success: true,
      message: "Local validation successful (activated)",
    };
  } catch (err) {
    // if the file does not exist, assume the license is not activated
  }
  await localDeactivate();
  return {
    success: true,
    message: "Local validation successful (not activated)",
  };
}

/**
 * Validate the activation code with the server.
 */
async function remoteValidate() {
  const { activated, activationCode, deviceId } = licenseStatus;
  if (!activated) {
    return {
      success: true,
      message: "Validation successful (not activated)",
    };
  }
  try {
    if (deviceId === "*") {
      // if offline activation, deactivate if the server is reachable
      const response = await fetch(`${LICENSE_SERVER_URL}/ping`, {
        method: "POST",
      });
      if (response.ok) {
        await localDeactivate();
        return {
          success: false,
          message: "License deactivated (illegal offline use)",
        };
      }
    } else {
      // if online activation, validate the activation code with the server
      const response = await fetch(`${LICENSE_SERVER_URL}/validate`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          activation_code: activationCode,
        }),
      });
      if (response.ok) {
        const data = await response.json();
        if (data.success) {
          const decoded = await decodeValidationCode(data.validation_code);
          if (decoded.deviceId === deviceId) {
            return {
              success: true,
              message: "Validation successful (activated)",
            };
          }
        }
      }
      await localDeactivate();
      return {
        success: false,
        message: "License deactivated by server",
      };
    }
  } catch (err) {
    // if server is not accessible, assume the validation is success.
    return {
      success: true,
      message: "Validation successful (offline)",
    };
  }
}

async function localActivate(activationCode) {
  try {
    const filePath = path.join(app.getPath("userData"), LICENSE_FILE_NAME);
    await fs.writeFile(filePath, activationCode, "utf8");
  } catch (err) {
    console.error("Local activation failed:", err);
  }
  const decoded = await decodeActivationCode(activationCode);
  licenseStatus = {
    activated: true,
    name: decoded.name,
    product: decoded.product,
    edition: decoded.edition,
    productDisplayName: getProductDisplayName(decoded.product, decoded.edition),
    deviceId: decoded.deviceId,
    licenseKey: decoded.licenseKey,
    activationCode: activationCode,
    trial: false,
    trialDaysLeft: 0,
  };
}

async function remoteActivate(licenseKey) {
  try {
    // request to activate this machine with the license key
    const deviceId = await getDeviceId();
    const response = await fetch(`${LICENSE_SERVER_URL}/activate`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        device_id: deviceId,
        license_key: licenseKey,
      }),
    });
    if (!response.ok) {
      const data = await response.json();
      if (!data.success) {
        return {
          success: false,
          message: data.error || "Activation failed",
        };
      }
      return {
        success: false,
        message: "Activation failed",
      };
    }

    // extract the activation code from the response
    const data = await response.json();
    if (!data.success) {
      return {
        success: false,
        message: data.error || "Activation failed",
      };
    }
    const activationCode = data.activation_code;

    // validate the activation code
    const decoded = await decodeActivationCode(activationCode);
    if (decoded.deviceId !== deviceId || decoded.licenseKey !== licenseKey) {
      return {
        success: false,
        message: "Invalid activation code",
      };
    }

    // local activation
    await localActivate(activationCode);
  } catch (err) {
    console.error("Remote activation failed", err);
    return {
      success: false,
      message: "Activation failed",
    };
  }

  return {
    success: true,
    message: "Activation successful",
  };
}

async function localDeactivate() {
  try {
    const filePath = path.join(app.getPath("userData"), LICENSE_FILE_NAME);
    await fs.unlink(filePath);
  } catch (err) {
    // if the file does not exist, ignore the error
  }
  licenseStatus = {
    activated: false,
    name: null,
    product: null,
    edition: null,
    productDisplayName: null,
    deviceId: null,
    licenseKey: null,
    activationCode: null,
    trial: false,
    trialDaysLeft: 0,
  };
  await checkTrialMode();
}

async function remoteDeactivate() {
  try {
    const deviceId = await getDeviceId();
    const response = await fetch(`${LICENSE_SERVER_URL}/deactivate`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        device_id: deviceId,
      }),
    });
    if (response.ok) {
      const data = await response.json();
      if (data.success) {
        await localDeactivate();
        // eslint-disable-next-line no-useless-return
        return {
          success: true,
          message: "Deactivation successful",
        };
      }
    }
  } catch (err) {
    console.error("Remote deactivation failed", err);
    return {
      success: false,
      message: "Deactivation failed",
    };
  }
  return {
    success: false,
    message: "Deactivation failed",
  };
}

function getLicenseStatus() {
  return licenseStatus;
}

function getProductDisplayName(product, edition) {
  let displayName = PRODUCT_DISPLAY_NAME[product];
  if (edition) {
    displayName += " " + EDITION_DISPLAY_NAME[edition];
  }
  return displayName;
}

module.exports = {
  getDeviceId,
  localValidate,
  remoteValidate,
  localActivate,
  remoteActivate,
  localDeactivate,
  remoteDeactivate,
  getLicenseStatus,
};
