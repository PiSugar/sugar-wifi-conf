//index.js
//获取应用实例

const regeneratorRuntime = require("../../lib/regenerator-runtime.js");
const utils = require("../../utils/util.js");
const UUID = require("../../lib/sugar-uuid.js");

const { ab2str, str2ab, buf2hex, apiAsync, hexCharCodeToStr } = utils;

const app = getApp();

Page({
  data: {
    piList: [],
  },
  showDetail(e) {
    wx.navigateTo({
      url:
        "/pages/detail/detail?deviceId=" +
        e.currentTarget.dataset.id +
        "&deviceName=" +
        e.currentTarget.dataset.name,
    });
  },
  openManual() {
    wx.navigateTo({
      url: "/pages/install/install",
    });
  },
  onLoad() {
    wx.startPullDownRefresh({
      success: function () {
        console.log("Enable Refresh");
      },
    });
  },
  onPullDownRefresh: async function () {
    console.log("refresh");
    wx.stopPullDownRefresh();
    const page = this;
    wx.onBluetoothDeviceFound(function (res) {
      let deviceId = res.devices[0].deviceId;
      let name = res.devices[0].name.toLowerCase();
      if (page.data.piList.filter((i) => i.deviceId === deviceId).length > 0)
        return; //排除重复蓝牙ID
      console.log(res);
      console.log("A Raspberry Pi is found! " + deviceId);
      page.data.piList.push({
        deviceId: deviceId,
        name: name,
      });
      console.log(page.data.piList);
      page.setData({
        piList: page.data.piList,
      });
      page.getCacheNames();
    });

    let openResult = await apiAsync("openBluetoothAdapter");
    console.log(openResult);
    let userSetting = await apiAsync("getSetting");
    console.log(userSetting);
    if (
      openResult.errMsg.indexOf("fail") > 0 &&
      openResult.errMsg.indexOf("fail already opened") < 0
    ) {
      wx.showModal({
        title: "蓝牙无法使用",
        content: "请确保手机蓝牙已经开启。",
        success(res) {
          if (res.confirm) {
            console.log("用户点击确定");
          } else if (res.cancel) {
            console.log("用户点击取消");
          }
        },
      });
    }
    let discoverResult = await apiAsync("startBluetoothDevicesDiscovery", {
      services: [UUID.SERVICE_ID],
    });
    console.log(discoverResult.errMsg);
  },
  onShow: async function () {
    console.log("show");
    wx.showShareMenu({
      withShareTicket: true,
    });
    this.getCacheNames();
  },
  getCacheNames() {
    let piList = this.data.piList.map((item) => {
      let cacheName = item.name;
      try {
        cacheName = wx.getStorageSync("name-" + item.deviceId);
      } catch (e) {
        // Do something when catch error
      }
      item.name = cacheName ? cacheName : item.name;
      return item;
    });
    this.setData({
      piList: piList,
    });
  },
});
