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

const fs = require("fs");
const path = require("path");
const Mustache = require("mustache");
const ViewUtils = require("../utils/view-utils");
const Strings = require("../strings");

const dialogTemplate = fs.readFileSync(
  path.join(__dirname, "../static/html-contents/icon-picker-dialog.html"),
  "utf8",
);

/**
 * @private
 * Icon Picker Dialog
 */
class IconPickerDialog {
  constructor() {
    this.autoComplete = null;
    this.selected = null;
    this.allItems = [];
    this.filteredItems = [];
    this.displayedItems = [];
    this.itemsPerPage = 50;
    this.currentPage = 0;
    this.isLoading = false;
  }

  /**
   * Convert file to item object
   * @private
   * @param {string} basePath
   * @param {string} file
   * @return {Object}
   */
  _toDataItem(basePath, file) {
    return {
      id: file,
      icon: path.join(basePath, file),
      text: file
        .replaceAll(".svg", "")
        .replaceAll("-", " ")
        .replaceAll("_", " "),
    };
  }

  /**
   * Initialize all items
   * @param {string} basePath
   * @param {string[]} files
   */
  initializeAllItems(basePath, files) {
    this.allItems = files.map((file) => this._toDataItem(basePath, file));
  }

  /**
   * Filter items based on search term
   * @param {string} filter
   */
  filterItems(filter = "") {
    if (filter.trim().length > 0) {
      this.filteredItems = this.allItems.filter(
        (item) =>
          item.text.toLowerCase().includes(filter.toLowerCase()) ||
          item.id.toLowerCase().includes(filter.toLowerCase()),
      );
    } else {
      this.filteredItems = [...this.allItems];
    }
    this.currentPage = 0;
    this.displayedItems = [];
  }

  /**
   * Load next page of items
   * @return {Object[]} items for the next page
   */
  loadNextPage() {
    const startIndex = this.currentPage * this.itemsPerPage;
    const endIndex = startIndex + this.itemsPerPage;
    const nextPageItems = this.filteredItems.slice(startIndex, endIndex);

    this.displayedItems.push(...nextPageItems);
    this.currentPage++;

    return nextPageItems;
  }

  /**
   * Check if there are more items to load
   * @return {boolean}
   */
  hasMoreItems() {
    return this.currentPage * this.itemsPerPage < this.filteredItems.length;
  }

  /**
   * Render list item HTML
   * @param {Object} item
   * @return {string}
   */
  renderListItem(item) {
    return `<li class="list-item" data-id="${item.id}">
      <img src="${item.icon}" loading="lazy" alt="${item.text}">
      <div class="item-text">${item.text}</div>
    </li>`;
  }

  /**
   * Show Icon Picker Dialog.
   * @param {string} baseDir
   * @return {Dialog}
   */
  showDialog(baseDir) {
    var context = {
      Strings,
      title: "Select Icon",
    };
    var dialog = app.dialogs.showModalDialogUsingTemplate(
      Mustache.render(dialogTemplate, context),
      true,
      ($dlg) => {
        $dlg.data("returnValue", this.selected);
      },
    );

    var $dlg = dialog.getElement();
    var $wrapper = $dlg.find(".listview-wrapper");
    var $listview = $dlg.find(".listview");
    ViewUtils.addScrollerShadow($wrapper, null, true);

    // icon files
    const basePath = path.join(__dirname, "../../resources/assets/", baseDir);
    const files = fs.readdirSync(basePath);

    // Initialize data
    this.initializeAllItems(basePath, files);
    this.filterItems("");

    // setup search
    const $input = $dlg.find(".icon-search-input");
    $input.keyup((e) => {
      const val = e.target.value;
      this.handleSearch(val, $listview);
    });
    $input.focus();

    // Setup infinite scroll
    this.setupInfiniteScroll($listview, $wrapper);

    // Load initial items
    this.loadMoreItems($listview);

    // Handle item selection
    $listview.on("click", ".list-item", (e) => {
      const $item = $(e.currentTarget);
      const itemId = $item.data("id");

      // Remove previous selection
      $listview.find(".list-item").removeClass("selected");

      // Add selection to clicked item
      $item.addClass("selected");

      // Store selected value
      this.selected = itemId;
    });

    // Handle double click to select and close dialog
    $listview.on("dblclick", ".list-item", (e) => {
      const $item = $(e.currentTarget);
      const itemId = $item.data("id");

      // Set selected value
      this.selected = itemId;
      // $dlg.data("returnValue", this.selected);

      // Close dialog with OK result
      dialog.close("ok");
    });

    return dialog;
  }

  /**
   * Handle search functionality
   * @param {string} filter
   * @param {jQuery} $listview
   */
  handleSearch(filter, $listview) {
    this.filterItems(filter);
    $listview.empty();
    this.loadMoreItems($listview);
  }

  /**
   * Setup infinite scroll functionality
   * @param {jQuery} $listview
   * @param {jQuery} $wrapper
   */
  setupInfiniteScroll($listview, $wrapper) {
    const self = this;

    $wrapper.on("scroll", function () {
      const scrollTop = $(this).scrollTop();
      const scrollHeight = $(this)[0].scrollHeight;
      const height = $(this).height();

      // Load more items when near bottom (within 100px)
      if (scrollTop + height >= scrollHeight - 100) {
        if (!self.isLoading && self.hasMoreItems()) {
          self.loadMoreItems($listview);
        }
      }
    });
  }

  /**
   * Load more items into the listview
   * @param {jQuery} $listview
   */
  loadMoreItems($listview) {
    if (this.isLoading) return;

    this.isLoading = true;

    // Simulate loading delay for better UX
    setTimeout(() => {
      const nextPageItems = this.loadNextPage();

      if (nextPageItems.length > 0) {
        const itemsHtml = nextPageItems
          .map((item) => this.renderListItem(item))
          .join("");
        $listview.append(itemsHtml);
      }

      this.isLoading = false;
    }, 50);
  }
}

module.exports = IconPickerDialog;
