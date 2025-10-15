let util = require('util')
let bleno = require('@abandonware/bleno')
let UUID = require('./sugar-uuid')
let config = require('./config')

let ServiceNameCharacteristic = require('./characteristics/service-name')
let DeviceModelCharacteristic = require('./characteristics/device-model')
let WifiNameCharacteristic = require('./characteristics/wifi-name')
let IpAddressCharacteristic = require('./characteristics/ip-address')
let InputCharacteristic = require('./characteristics/input-notify').InputCharacteristic
let InputCharacteristicSep = require('./characteristics/input-notify').InputCharacteristicSep
let NotifyMassageCharacteristic = require('./characteristics/input-notify').NotifyMassageCharacteristic
let CustomCharacteristics = require('./characteristics/custom-info')
let CustomCommandCharacteristics = require('./characteristics/custom-command')

let BlenoPrimaryService = bleno.PrimaryService

function wait (sec) {
  return new Promise(function (resolve, reject) {
    setTimeout(function () {
      resolve(true)
    }, sec * 1000)
  })
}

async function startBLE () {
  console.log('Wait 10 seconds')
  await wait(10)
  // console.log('check bluetooth')
  // console.log(execSync('dmesg |grep -i Bluetooth').toString())
  console.log('Bleno starting...')

  bleno.on('stateChange', function(state) {
    console.log('on -> stateChange: ' + state + ', address = ' + bleno.address)
    if (state === 'poweredOn') {
      bleno.startAdvertising(config.name, [ UUID.SERVICE_ID ])
    } else {
      bleno.stopAdvertising()
    }
  })

// Linux only events /////////////////
  bleno.on('accept', function(clientAddress) {
    console.log('on -> accept, client: ' + clientAddress)
    bleno.updateRssi()
  })

  bleno.on('disconnect', function(clientAddress) {
    console.log('on -> disconnect, client: ' + clientAddress)
  })

  bleno.on('rssiUpdate', function(rssi) {
    console.log('on -> rssiUpdate: ' + rssi)
  })
//////////////////////////////////////

  bleno.on('mtuChange', function(mtu) {
    console.log('on -> mtuChange: ' + mtu)
  })

  bleno.on('advertisingStart', function(error) {
    console.log('on -> advertisingStart: ' + (error ? 'error ' + error : 'success'))
    if (!error) {
      bleno.setServices([
        new BlenoPrimaryService({
          uuid: UUID.SERVICE_ID,
          characteristics: [
            ServiceNameCharacteristic,
            DeviceModelCharacteristic,
            WifiNameCharacteristic,
            IpAddressCharacteristic,
            InputCharacteristic,
            InputCharacteristicSep,
            NotifyMassageCharacteristic,
            ...CustomCharacteristics,
            ...CustomCommandCharacteristics
          ]
        })
      ])
    }
  })

  bleno.on('advertisingStop', function() {
    console.log('on -> advertisingStop')
  })

  bleno.on('servicesSet', function(error) {
    console.log('on -> servicesSet: ' + (error ? 'error ' + error : 'success'))
  })
}

startBLE()
