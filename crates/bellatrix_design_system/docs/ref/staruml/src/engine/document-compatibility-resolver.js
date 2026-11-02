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

const { DOCUMENT_VERSION } = require("../core/core");

function sendToBack(view) {
  const parentView = view._parent;
  if (parentView instanceof type.Diagram) {
    parentView.ownedViews.splice(parentView.ownedViews.indexOf(view), 1);
    parentView.ownedViews.unshift(view);
  }
}

function compatibilityResolver(project) {
  if (project.documentVersion < 1) {
    console.log("Old document version detected. Migrating...");

    // 1. Send to back for all use case subjects
    const useCaseSubjectViews = app.repository.getInstancesOf(
      "UMLUseCaseSubjectView",
    );
    useCaseSubjectViews.forEach((view) => {
      if (view) {
        sendToBack(view);
      }
    });

    project.documentVersion = DOCUMENT_VERSION;
    console.log("Project has been migrated to the latest document version");
  }
}

exports.compatibilityResolver = compatibilityResolver;
