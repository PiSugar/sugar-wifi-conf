// pages/detail/detail.js
const regeneratorRuntime = require('../../lib/regenerator-runtime.js')
const utils = require('../../utils/util.js')
const UUID = require('../../lib/sugar-uuid.js')
const {
  ab2str,
  str2ab,
  str2abs,
  buf2hex,
  apiAsync,
  hexCharCodeToStr
} = utils

const page = Page({

  /**
   * 页面的初始数据
   */
  data: {
    nameMode: false,
    deviceName: 'raspberrypi',
    deviceId: '...',
    deviceModel: '...',
    wifiName: '...',
    ipAddress: '...',
    notifyMessage: '',
    ssid: '',
    password: '',
    key: 'pisugar',
    currentTab: 'wifi',
    settingLock: false,
    commandLock: false,
    customInfo: [],
    customCommands: [{
      label: 'demo',
      uuidLable: 's'
    }, {
        label: 'demo2',
        uuidLable: 's'
      }],
    responseText: '',
    responseDone: true
  },
  onPullDownRefresh: async function () {
    wx.stopPullDownRefresh();
  },
  /**
   * 生命周期函数--监听页面加载
   */
  onLoad: async function (option) {
    wx.hideShareMenu()
    wx.showLoading({
      title: '蓝牙连接中...',
    })
    try {
      const key = wx.getStorageSync('key')
      const ssid = wx.getStorageSync('ssid')
      if (key && ssid) {
        this.setData({
          key: key,
          ssid: ssid
        })
      }
    } catch (e) {
      // Do something when catch error
    }
    const page = this
    wx.onBLECharacteristicValueChange(function (res) {
      let uuid = res.characteristicId
      let charName = ''
      for (let i in UUID) {
        if (UUID[i] === uuid) {
          charName = i
        }
      }
      switch (charName) {
        case 'DEVICE_MODEL':
          page.setData({
            deviceModel: ab2str(res.value)
          })
          break;
        case 'WIFI_NAME':
          page.setData({
            wifiName: ab2str(res.value)
          })
          break;
        case 'IP_ADDRESS':
          page.setData({
            ipAddress: ab2str(res.value)
          })
          break;
        case 'NOTIFY_MESSAGE':
          page.setData({
            notifyMessage: ab2str(res.value)
          })
          wx.showModal({
            title: '蓝牙消息',
            content: ab2str(res.value)
          })
          break;
        case 'CUSTOM_COMMAND_NOTIFY':
          page.setResponse(ab2str(res.value))
          break;
        default:
          page.customRes(res.characteristicId, ab2str(res.value))
          break;
      }
      // console.log(charName, ab2str(res.value))
    })
    console.log(option)
    let deviceId = option.deviceId
    let deviceName = option.deviceName !== '' ? option.deviceName : 'Raspberry Pi'
    let cacheName
    try {
      cacheName = wx.getStorageSync('name-'+deviceId)
    } catch (e) {
      // Do something when catch error
    }
    this.setData({
      deviceId: deviceId,
      deviceName: cacheName ? cacheName : deviceName
    })
    //createConnection
    if (deviceId) {
      let res = await apiAsync('createBLEConnection', { deviceId })
      if (res.errMsg !== 'createBLEConnection:ok') {
        wx.showToast({
          title: '连接失败',
          icon: 'cancel',
          duration: 2000
        })
        return
      }
      res = await apiAsync('getBLEDeviceServices', { deviceId })
      if (res.errMsg !== 'getBLEDeviceServices:ok') {
        wx.showToast({
          title: '获取服务失败',
          icon: 'cancel',
          duration: 2000
        })
        return
      }
      wx.hideLoading();
      let serviceId = UUID.SERVICE_ID
      res = await apiAsync('getBLEDeviceCharacteristics', { deviceId, serviceId })
      console.log(res)
      let deviceModelCharId = UUID.DEVICE_MODEL
      getDeviceModel(deviceId)
      subscribeWifiName(deviceId)
      subscribeIpAddress(deviceId)
      subscribeNotifyMassage(deviceId)
      if (res.characteristics) {
        let customInfo = []
        let customCommands = []
        res.characteristics.map(item => {
          if (item.uuid.indexOf(UUID.CUSTOM_INFO_LABEL) > 0) {
            customInfo.push(
              {
                uuidLabel: item.uuid,
                uuidValue: item.uuid.replace(UUID.CUSTOM_INFO_LABEL, UUID.CUSTOM_INFO),
                label: '...',
                value: '...'
              }
            )
          }
          if (item.uuid.indexOf(UUID.CUSTOM_COMMAND_LABEL) > 0) {
            customCommands.push(
              {
                uuidLabel: item.uuid,
                label: '...'
              }
            )
          }
        })
        console.log('custom items')
        console.log(customInfo, customCommands)
        this.setData(
          {
            customInfo: customInfo,
            customCommands:customCommands
          }
        )
        for (let i in customInfo) {
          getCustomLabel(deviceId, customInfo[i].uuidLabel)
          subscribeCustomValue(deviceId, customInfo[i].uuidValue)
        }
        for (let i in customCommands) {
          getCustomLabel(deviceId, customCommands[i].uuidLabel)
        }
        if (customCommands.length) {
          subscribeCustomValue(deviceId, UUID.CUSTOM_COMMAND_NOTIFY)
        }
      }
    }
  },
  onUnload() {
    let deviceId = this.data.deviceId
    if (deviceId !== '') {
      apiAsync('closeBLEConnection', { deviceId })
    }
  },
  customRes(cId, str) {
    let customInfo = this.data.customInfo.map(item => {
      if (item.uuidLabel === cId) item.label = str
      if (item.uuidValue === cId) item.value = str
      return item
    })
    let customCommands = this.data.customCommands.map(item => {
      if (item.uuidLabel === cId) item.label = str
      return item
    })
    this.setData({
      customInfo: customInfo,
      customCommands: customCommands
    })
  },
  changeNameMode () {
    this.setData({
      nameMode: true
    })
  },
  changeDeviceName (e) {
    this.setData({
      deviceName: e.detail.value,
      nameMode: false
    })
    try {
      wx.setStorageSync('name-' + this.data.deviceId, this.data.deviceName)
    } catch (e) { }
  },
  bindSelectWifiTab () {
    this.setData({
      currentTab: 'wifi'
    })
  },
  bindSelectCommandTab() {
    this.setData({
      currentTab: 'command'
    })
  },
  bindSsid (e) {
    this.setData({
      ssid: e.detail.value
    })
  },
  bindPass (e) {
    this.setData({
      password: e.detail.value
    })
  },
  bindKey (e) {
    this.setData({
      key: e.detail.value
    })
  },
  async getWifi () {
    let res = await apiAsync('getConnectedWifi')
    if (res.wifi) {
      this.setData({
        ssid: res.wifi.SSID
      })
    } else {
      wx.showModal({
        content: '无法获取ssid，请将手机连接到wifi'
      })
    }
  },
  async setWifi () {
    const page = this
    if (this.data.settingLock) return
    wx.showLoading({
      title: '设置中',
      mask: true
    })
    this.setData({
      settingLock: true
    })
    let key = this.data.key.trim().replace(/\|/g, "*")
    let ssid = this.data.ssid.trim().replace(/\|/g, "*")
    let password = this.data.password.trim().replace(/\|/g, "*")
    let errMsg
    if (ssid === '') {
      errMsg = 'SSID不能为空'
    }
    if (password === '') {
      errMsg = '密码不能为空'
    }
    if (errMsg) {
      wx.showModal({
        title: '',
        content: errMsg
      })
      wx.hideLoading()
      return
    }
    try {
      wx.setStorageSync('key', key)
      wx.setStorageSync('ssid', ssid)
    } catch (e) { }
    console.log('start sending')
    await inputWifiAndroid(this.data.deviceId, `${key}%&%${ssid}%&%${password}`)
    setTimeout(function () {
      page.setData({
        settingLock: false
      })
    }, 1000)
    wx.hideLoading()
  },
  async sendCommand (e) {
    const page = this
    if (this.data.commandLock) return
    wx.showLoading({
      title: '正在发送命令',
      mask: true
    })
    this.setData({
      commandLock: true
    })
    let key = this.data.key.trim().replace(/\|/g, "*")
    try {
      wx.setStorageSync('key', key)
      wx.setStorageSync('ssid', ssid)
    } catch (e) { }
    await inputCommand(this.data.deviceId, `${key}%&%${e.target.dataset.uuid}`)
    setTimeout(function () {
      page.setData({
        commandLock: false
      })
    }, 1000)
    wx.hideLoading()
  },
  setResponse (str) {
    if (this.data.responseDone) {
      this.data.responseDone = false
      this.data.responseText = ''
    }
    this.data.responseText += str
    if (this.data.responseText.indexOf('&#&') >= 0) {
      this.data.responseText = this.data.responseText.replace('&#&', '')
      this.data.responseDone = true
    }
    this.setData({
      responseText: this.data.responseText,
      responseDone: this.data.responseDone
    })
  }
})


function getServiceName(deviceId) {
  return apiAsync('readBLECharacteristicValue', { deviceId, serviceId: UUID.SERVICE_ID, characteristicId: UUID.SERVICE_NAME })
}

function getDeviceModel(deviceId) {
  return apiAsync('readBLECharacteristicValue', { deviceId, serviceId: UUID.SERVICE_ID, characteristicId: UUID.DEVICE_MODEL })
}

function getIpAddress(deviceId) {
  return apiAsync('readBLECharacteristicValue', { deviceId, serviceId: UUID.SERVICE_ID, characteristicId: UUID.IP_ADDRESS })
}

function subscribeIpAddress(deviceId) {
  return apiAsync('notifyBLECharacteristicValueChange', { deviceId, serviceId: UUID.SERVICE_ID, characteristicId: UUID.IP_ADDRESS, state: true })
}

function getWifiName(deviceId) {
  return apiAsync('readBLECharacteristicValue', { deviceId, serviceId: UUID.SERVICE_ID, characteristicId: UUID.WIFI_NAME })
}

function subscribeWifiName(deviceId) {
  return apiAsync('notifyBLECharacteristicValueChange', { deviceId, serviceId: UUID.SERVICE_ID, characteristicId: UUID.WIFI_NAME, state: true })
}

function getCustomLabel(deviceId, uuidLabel) {
  return apiAsync('readBLECharacteristicValue', { deviceId, serviceId: UUID.SERVICE_ID, characteristicId: uuidLabel })
}

function subscribeCustomValue(deviceId, uuidValue) {
  return apiAsync('notifyBLECharacteristicValueChange', { deviceId, serviceId: UUID.SERVICE_ID, characteristicId: uuidValue, state: true })
}

function inputWifi(deviceId, string) {
  const arrayBuffer = str2ab(string)
  return apiAsync('writeBLECharacteristicValue', { deviceId, serviceId: UUID.SERVICE_ID, characteristicId: UUID.INPUT, value: arrayBuffer })
}

let inputWifiAndroid = async function (deviceId, string) {
  const arrayBufferArray = str2abs(string + '&#&')
  for (let i in arrayBufferArray) {
    console.log('sending...' + i)
    await sleep(0.4)
    console.log(arrayBufferArray[i])
    await apiAsync('writeBLECharacteristicValue', { deviceId, serviceId: UUID.SERVICE_ID, characteristicId: UUID.INPUT_SEP, value: arrayBufferArray[i]})
  }
  return true
}

let inputCommand = async function (deviceId, string) {
  const arrayBufferArray = str2abs(string + '&#&')
  for (let i in arrayBufferArray) {
    console.log('sending...' + i)
    await sleep(0.4)
    console.log(arrayBufferArray[i])
    await apiAsync('writeBLECharacteristicValue', { deviceId, serviceId: UUID.SERVICE_ID, characteristicId: UUID.CUSTOM_COMMAND_INPUT, value: arrayBufferArray[i] })
  }
  return true
}

function subscribeNotifyMassage(deviceId) {
  return apiAsync('notifyBLECharacteristicValueChange', { deviceId, serviceId: UUID.SERVICE_ID, characteristicId: UUID.NOTIFY_MESSAGE, state: true })
}

function sleep(sec) {
  console.log('wait for a moment...')
  return new Promise(function (resolve, reject) {
    setTimeout(function () {
      resolve(true)
    }, sec * 1000)
  })
}