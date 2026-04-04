// pages/install/install.js
Page({

  /**
   * 页面的初始数据
   */
  data: {
    manual: `#适用于带有蓝牙的树莓派型号(3B/3B+/zero w等)\n\ngit clone https://github.com/PiSugar/sugar-wifi-conf.git\n\nsudo -s. ./sugar-wifi-conf/wificonfig.sh\n\n#重启后即可使用`
  },

  copyText: function () {
    wx.setClipboardData({
      data: this.data.manual,
      success: function (res) {
        wx.getClipboardData({
          success: function (res) {
            wx.showToast({
              title: '已复制到剪切板'
            })
          }
        })
      }
    })
  },

  onPullDownRefresh: function () {
    wx.stopPullDownRefresh();
  },

  /**
   * 生命周期函数--监听页面加载
   */
  onLoad: function (options) {

  },

  /**
   * 生命周期函数--监听页面初次渲染完成
   */
  onReady: function () {

  },

  /**
   * 生命周期函数--监听页面显示
   */
  onShow: function () {

  },

  /**
   * 生命周期函数--监听页面隐藏
   */
  onHide: function () {

  },

  /**
   * 生命周期函数--监听页面卸载
   */
  onUnload: function () {

  },

  /**
   * 页面相关事件处理函数--监听用户下拉动作
   */
  onPullDownRefresh: function () {

  },

  /**
   * 页面上拉触底事件的处理函数
   */
  onReachBottom: function () {

  },

  /**
   * 用户点击右上角分享
   */
  onShareAppMessage: function () {

  }
})