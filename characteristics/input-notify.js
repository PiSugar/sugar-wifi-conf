const execSync = require('child_process').execSync
let util = require('util')
let bleno = require('bleno')
let UUID = require('../sugar-uuid')
let config = require('../config')
const fs = require('fs')
const conf_path = '/etc/wpa_supplicant/wpa_supplicant.conf'
const iface_path = '/etc/network/interfaces'
const concatTag = '%&%'
const endTag = '&#&'

let argv = process.argv
if (argv.length > 2) config.key = process.argv[2]

let BlenoCharacteristic = bleno.Characteristic
let message = ''
let messageTimestamp = 0

// Input

let InputCharacteristic = function() {
  InputCharacteristic.super_.call(this, {
    uuid: UUID.INPUT,
    properties: ['write', 'writeWithoutResponse']
  })
}

util.inherits(InputCharacteristic, BlenoCharacteristic)

InputCharacteristic.prototype.onWriteRequest = function(data, offset, withoutResponse, callback) {
  console.log('InputCharacteristic write request: ' + data.toString() + ' ' + offset + ' ' + withoutResponse)
  let inputArray = data.toString().split(concatTag)
  if (inputArray && inputArray.length < 3) {
    console.log('Wrong input syntax.')
    setMessage('Wrong input syntax.')
    callback(this.RESULT_SUCCESS)
    return
  }
  if (inputArray[0] !== config.key){
    console.log('Wrong input key.')
    setMessage('Wrong input key.')
    callback(this.RESULT_SUCCESS)
    return
  }
  let ssid = inputArray[1]
  let password = inputArray[2]
  let result = setWifi(ssid, password)
  callback(this.RESULT_SUCCESS)
}


// Input android

let separateInputString = ''
let separateInputStringCopy = ''
let lastChangeTime = 0
let clearTime = 5000

setInterval(function () {
  if (separateInputStringCopy !== separateInputString) {
    separateInputStringCopy = separateInputString
    lastChangeTime = new Date().getTime()
  } else if (new Date().getTime() - lastChangeTime > clearTime && separateInputString !== '') {
    lastChangeTime = new Date().getTime()
    separateInputStringCopy = ''
    separateInputString = ''
    console.log('clear separateInputString')
  }
}, 1000)

let InputCharacteristicSep = function() {
  InputCharacteristicSep.super_.call(this, {
    uuid: UUID.INPUT_SEP,
    properties: ['write', 'writeWithoutResponse']
  })
}

util.inherits(InputCharacteristicSep, BlenoCharacteristic)

InputCharacteristicSep.prototype.onWriteRequest = function(data, offset, withoutResponse, callback) {
  console.log('InputCharacteristicSep write request: ' + data.toString() + ' ' + offset + ' ' + withoutResponse)
  separateInputString += data.toString()
  let isLast = separateInputString.indexOf(endTag) >= 0
  if (isLast) {
    separateInputString = separateInputString.replace(endTag, '')
    let inputArray = separateInputString.split(concatTag)
    lastChangeTime = new Date().getTime()
    separateInputStringCopy = ''
    separateInputString = ''
    if (inputArray && inputArray.length < 3) {
      console.log('Invalid syntax.')
      setMessage('Invalid syntax.')
      callback(this.RESULT_SUCCESS)
      return
    }
    if (inputArray[0] !== config.key){
      console.log('Invalid key.')
      setMessage('Invalid key.')
      callback(this.RESULT_SUCCESS)
      return
    }
    let ssid = inputArray[1]
    let password = inputArray[2]
    let result = setWifi(ssid, password)
  }
  callback(this.RESULT_SUCCESS)
}


// NotifyMassage

let NotifyMassageCharacteristic = function() {
  NotifyMassageCharacteristic.super_.call(this, {
    uuid: UUID.NOTIFY_MESSAGE,
    properties: ['notify']
  })
}

util.inherits(NotifyMassageCharacteristic, BlenoCharacteristic)

NotifyMassageCharacteristic.prototype.onSubscribe = function(maxValueSize, updateValueCallback) {
  console.log('NotifyMassageCharacteristic subscribe')
  this.timeStamp = messageTimestamp
  this.changeInterval = setInterval(function() {
    if (this.timeStamp === messageTimestamp) return
    let data = new Buffer(message)
    console.log('NotifyMassageCharacteristic update value: ' + message)
    updateValueCallback(data)
    this.timeStamp = messageTimestamp
  }.bind(this), 100)
}

NotifyMassageCharacteristic.prototype.onUnsubscribe = function() {
  console.log('NotifyMassageCharacteristic unsubscribe')

  if (this.changeInterval) {
    clearInterval(this.changeInterval)
    this.changeInterval = null
  }
}

NotifyMassageCharacteristic.prototype.onNotify = function() {
  console.log('NotifyMassageCharacteristic on notify')
}

async function setWifi (input_ssid, input_password) {
  let data = fs.readFileSync(conf_path, 'utf8')
  let wifiRegx = /(network={[^\}]+})/g
  let ssidRegx = /ssid="([^"]*)"/
  let priorityRegx = /priority=([\d]*)/
  let wifiMatch = data.match(wifiRegx)
  let wifiArray = []
  let maxPriority = 0
  if (wifiMatch) {
    for (let i in wifiMatch) {
      let str = wifiMatch[i]
      let ssid = str.match(ssidRegx)
      ssid = ssid ? ssid[1] : ''
      let priority = str.match(priorityRegx)
      priority = priority ? priority[1] : 0
      maxPriority = Math.max(maxPriority, priority)
      if (input_ssid !== ssid) {
        wifiArray.push(str)
      }
      data = data.replace(wifiMatch[i], '')
    }
  }
  let prefix = data.replace('Country=', 'country=')
  wifiArray.push(`network={\n\t\tssid="${input_ssid}"\n\t\tscan_ssid=1\n\t\tpsk="${input_password}"\n\t\tpriority=${maxPriority+1}\n\t}`)
  let content = `${prefix}\n\t${wifiArray.join('\n\t')}`
  fs.writeFileSync(conf_path, content)
  // check if wlan0 available, otherwise let reboot
  if (!isWlan0Ok()) {
    setMessage('OK. Please reboot.')
    return
  }
  try{
    execSync('killall wpa_supplicant')
  } catch (e) {
    console.log(e.toString())
  }
  let resMsg = ''
  let maxTryTimes = 10
  while (maxTryTimes > 0) {
    // try every 2 second
    await sleep(2)
    try{
      let msg = execSync('wpa_supplicant -B -iwlan0 -c/etc/wpa_supplicant/wpa_supplicant.conf')
      resMsg = msg.toString()
      break
    } catch (e) {
      console.log(e)
      resMsg = 'Commond failed.'
    }
    maxTryTimes--
  }
  setMessage(maxTryTimes.toString() + ' ' + resMsg)
}

function isWlan0Ok() {
  let data = fs.readFileSync(iface_path, 'utf8')
  let rawContent = data.split('\n')
  let foundWlan0 = false
  let isOk = true
  for (const i in rawContent) {
    let line = rawContent[i].trim()
    if (foundWlan0 && line.indexOf('interface ') >=0 && line.indexOf('#') !== 0) {
      foundWlan0 = false
    }
    if (line.indexOf('interface wlan0') >=0 && line.indexOf('#') !== 0) {
      foundWlan0 = true
    }
    if (foundWlan0 && line.indexOf('nohook wpa_supplicant') >=0 && line.indexOf('#') !== 0) {
      isOk = false
    }
  }
  console.log('Is wlan0 Ok ? ' + isOk)
  return isOk
}


function sleep (sec) {
  console.log('wait for a moment...')
  return new Promise(function(resolve, reject){
    setTimeout(function(){
      resolve(true)
    }, sec*1000)
  })
}

function setMessage (msg) {
  message = msg
  messageTimestamp = new Date().getTime()
}

module.exports = {
  InputCharacteristic,
  InputCharacteristicSep,
  NotifyMassageCharacteristic
}