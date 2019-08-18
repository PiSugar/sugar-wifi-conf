<template>
  <div id="app">
    <img class="logo" src="./assets/logo.svg">
    <div class="panel-container">
      <div v-if="!supportBluetooth" class="connect-panel panel">
        <div class="panel-title">Cannot find web bluetooth.</div>
        <p>Please make sure your device and browser support web bluetooth. Please visit <a href="https://github.com/WebBluetoothCG/web-bluetooth/blob/master/implementation-status.md" target="_blank">Here</a> to check web-bluetooth compatibility.</p>
      </div>
      <div v-if="!isConnected && supportBluetooth" class="connect-panel panel">
        <div class="panel-title">Connect your pi ...</div>
        <el-row>
          <el-button @click="connectDevice">Discover</el-button>
        </el-row>
      </div>
      <div v-if="isConnected" class="info-panel panel">
        <div class="panel-title">Device Information</div>
        <div v-for="item in infoList" :key="item.uuid" class="info-group">
          <div class="label">{{item.label}}</div>
          <div class="value">{{item.value}}</div>
        </div>
      </div>
      <div v-if="isConnected" class="wifi-panel panel">
        <div class="panel-title">Wifi Setting</div>
        <el-row>
          <el-input placeholder="Wifi name" v-model="ssid">
            <template slot="prepend">Wifi</template>
          </el-input>
        </el-row>
        <el-row>
          <el-input placeholder="Wifi password" v-model="password" type="password">
            <template slot="prepend">Psw</template>
          </el-input>
        </el-row>
        <el-row>
          <el-input placeholder="Key (Default: pisugar)" v-model="key">
            <template slot="prepend">Key</template>
          </el-input>
        </el-row>
        <el-row>
          <el-button @click="inputWifi" :disabled="wifiLock">Submit</el-button>
        </el-row>
      </div>
      <div v-if="isConnected" class="command-panel panel">
        <div class="panel-title">Custom Commands</div>
        <template v-if="commandList.length > 0">
          <el-row>
            <div v-for="item in commandList" :key="item.uuid" class="button-wrap">
              <el-button size="small" round @click="sendCommand(item.uuid)">{{item.label}}</el-button>
            </div>
          </el-row>
          <el-row>
            <el-input placeholder="Key (Default: pisugar)" v-model="key">
              <template slot="prepend">Key</template>
            </el-input>
          </el-row>
          <el-row>
            <el-input
              type="textarea"
              placeholder="Output"
              v-model="commandOutput"
              :rows="10"
            ></el-input>
          </el-row>
        </template>
        <p v-else>
          No custom command is found.
        </p>
      </div>
    </div>
    <div class="copyright">This Project is maintained by <a href="https://github.com/PiSugar/sugar-wifi-conf" target="_blank">PiSugar Kitchen</a>. Open source under GPL-3.0 license.</div>
  </div>
</template>

<script>
import { Loading } from 'element-ui';
import UUID from './sugar-uuid'
for (let i in UUID) {
  UUID[i] = UUID[i].toLowerCase()
}
export default {
  name: 'App',
  components: {

  },
  data () {
    return {
      supportBluetooth: false,
      isConnected: false,
      serverId: '',
      ssid: '',
      password: '',
      key: 'pisugar',
      wifiLock: false,
      characteristicsList: [],
      infoList: [],
      commandList: [],
      commandOutput: '',
      commandOutputShouldRefresh: false,
      loading: null,
      charLength: -1,
      customInfoCount: 0,
      customCommandCount: 0,
      isAndroid: navigator.userAgent.indexOf('Android') > -1 || navigator.userAgent.indexOf('Adr') > -1,
      isIphone: navigator.userAgent.indexOf('iPhone') > -1 || navigator.userAgent.indexOf('iphone') > -1
      // isIphone: true
    }
  },
  mounted () {
    this.supportBluetooth = window.navigator.bluetooth
  },
  methods: {
    connectDevice () {
      const that = this
      navigator.bluetooth.requestDevice({
        filters: [{
          services: [UUID.SERVICE_ID]
        }]
      })
        .then(device => {
          that.loading = Loading.service({ fullscreen: true })
          return device.gatt.connect()
        })
        .then(server => {
          that.serverId = server.device.id
          return server.getPrimaryService(UUID.SERVICE_ID)
        })
        .then(service => {
          if (this.isIphone) {
            // iOS webBLE does not support getCharacteristics()
            console.log('ios webBLE')
            return this.webBleConnect(service)
          } else {
            return service.getCharacteristics()
          }
        })
        .then(characteristics => {
          that.characteristicsList = characteristics
          that.isConnected = true
          that.loading.close()
          // console.log(that.characteristicsList)
        })
        .catch(console.log)
    },
    webBleConnect (service) {
      const that = this
      return new Promise(async resolve => {
        await service.getCharacteristic(UUID.PREFIX + UUID.CUSTOM_INFO_COUNT)
          .then(characteristic => characteristic.readValue())
          .then(i => i.buffer)
          .then(this.ab2str)
          .then(parseInt)
          .then(value => {
            console.log('custom-info-count ' + value)
            that.customInfoCount = value
          })
        await service.getCharacteristic(UUID.PREFIX + UUID.CUSTOM_COMMAND_COUNT)
          .then(characteristic => characteristic.readValue())
          .then(i => i.buffer)
          .then(this.ab2str)
          .then(parseInt)
          .then(value => {
            console.log('custom-command-count ' + value)
            that.customCommandCount = value
          })
        let customInfoList = []
        for (let index = 0; index < that.customInfoCount; index++) {
          let ending = (index + 1).toString(16)
          ending = '0'.repeat(4 - ending.length) + ending
          customInfoList.push(UUID.PREFIX + UUID.CUSTOM_INFO + ending)
          customInfoList.push(UUID.PREFIX + UUID.CUSTOM_INFO_LABEL + ending)
        }
        let customCommandList = []
        for (let index = 0; index < that.customCommandCount; index++) {
          let ending = (index + 1).toString(16)
          ending = '0'.repeat(4 - ending.length) + ending
          customCommandList.push(UUID.PREFIX + UUID.CUSTOM_COMMAND_LABEL + ending)
        }
        resolve(Promise.all([
          service.getCharacteristic(UUID.SERVICE_NAME),
          service.getCharacteristic(UUID.DEVICE_MODEL),
          service.getCharacteristic(UUID.WIFI_NAME),
          service.getCharacteristic(UUID.IP_ADDRESS),
          service.getCharacteristic(UUID.NOTIFY_MESSAGE),
          service.getCharacteristic(UUID.INPUT_SEP),
          service.getCharacteristic(UUID.CUSTOM_COMMAND_INPUT),
          service.getCharacteristic(UUID.CUSTOM_COMMAND_NOTIFY),
          ...customInfoList.map(i => service.getCharacteristic(i)),
          ...customCommandList.map(i => service.getCharacteristic(i))
        ]))
      })
    },
    ab2str (buf) {
      return String.fromCharCode.apply(null, new Uint8Array(buf));
    },
    getCharacteristic (uuid) {
      return this.characteristicsList.find(i => i.uuid === uuid)
    },
    subscribeCharacteristic (uuid) {
      return new Promise(async resolve => {
        this.getCharacteristic(uuid).addEventListener('characteristicvaluechanged', event => {
          if (event.target.uuid === UUID.NOTIFY_MESSAGE) {
            let msg = this.ab2str(event.target.value.buffer)
            const h = this.$createElement
            this.$notify({
              title: 'Error',
              message: h('i', { style: 'color: teal'}, msg)
            })
          } else if (event.target.uuid === UUID.CUSTOM_COMMAND_NOTIFY) {
            let msg = this.ab2str(event.target.value.buffer)
            if (this.commandOutputShouldRefresh) {
              this.commandOutputShouldRefresh = false
              this.commandOutput = ''
            }
            let output = this.commandOutput + msg
            if (output.endsWith('&#&')) {
              output = output.replace('&#&', '')
              this.commandOutputShouldRefresh = true
            }
            this.commandOutput = output
          } else {
            let value = this.ab2str(event.target.value.buffer)
            let char = this.infoList.find(i => i.uuid === uuid)
            let char_label = this.infoList.find(i => i.uuid_label === uuid)
            if (char) {
              char.value = value
            }
            if (char_label) {
              char_label.label = value
            }
          }
        })
        await this.getCharacteristic(uuid).startNotifications()
        resolve()
      })
    },
    readInfoCharacteristic (uuid) {
      return new Promise(resolve => {
        this.getCharacteristic(uuid).readValue()
          .then(i => i.buffer)
          .then(this.ab2str)
          .then((value) => {
            let char = this.infoList.find(i => i.uuid === uuid)
            let char_label = this.infoList.find(i => i.uuid_label === uuid)
            if (char) {
              char.value = value
            }
            if (char_label) {
              char_label.label = value
            }
            resolve()
          })
      })
    },
    readCommandLabel (uuid) {
      return new Promise(resolve => {
        this.getCharacteristic(uuid).readValue()
          .then(i => i.buffer)
          .then(this.ab2str)
          .then((label) => {
            this.commandList.find(i => i.uuid === uuid).label = label
            resolve()
          })
      })
    },
    inputWifi () {
      let key = this.key.trim().replace(/\|/g, "*")
      let ssid = this.ssid.trim().replace(/\|/g, "*")
      let password = this.password.trim().replace(/\|/g, "*")
      let errMsg
      if (ssid === '') {
        errMsg = 'SSID cannot be empty.'
      }
      if (password === '') {
        errMsg = 'Password cannot be empty.'
      }
      if (errMsg) {
        const h = this.$createElement
        this.$notify({
          title: 'Error',
          message: h('i', { style: 'color: teal'}, errMsg)
        })
        return
      }
      this.wifiLock = true
      setTimeout(() => {
        this.wifiLock = false
      }, 4000)
      let sendArray = this.str2abs(`${key}%&%${ssid}%&%${password}&#&`)
      this.sendSeparately(sendArray, UUID.INPUT_SEP)
    },
    sendCommand (uuid) {
      let sendArray = this.str2abs(`${this.key}%&%${uuid.slice(-4)}&#&`)
      this.sendSeparately(sendArray, UUID.CUSTOM_COMMAND_INPUT)
    },
    async sendSeparately (array, uuid) {
      for (const i in array) {
        await this.getCharacteristic(uuid).writeValue(array[i])
        await this.wait(0.4)
      }
    },
    async wait (sec) {
      return new Promise((resolve => {
        setTimeout(() => {
          resolve(true)
        }, 1000 * sec)
      }))
    },
    str2abs(str) {
      let val = ''
      for (let i = 0; i < str.length; i++) {
        if (val === '') {
          val = str.charCodeAt(i).toString(16)
        } else {
          val += ',' + str.charCodeAt(i).toString(16)
        }
      }
      let valArray = val.split(',')
      let bufferArray = []
      while (valArray.length > 0) {
        let value = valArray.splice(0, 20).join(',')
        bufferArray.push(new Uint8Array(value.match(/[\da-f]{2}/gi).map(function (h) {
          return parseInt(h, 16)
        })).buffer)
      }
      return bufferArray
    }
  },
  watch: {
    async isConnected (val, oldVal) {
      if (val === true && oldVal === false) {
        console.log('Connected!')
        this.infoList = []
        this.infoList.push({
          preset: true,
          uuid: '',
          label: 'Device ID',
          value: this.serverId
        })
        this.infoList.push({
          preset: true,
          uuid: UUID.DEVICE_MODEL,
          label: 'Model',
          value: ' '
        })
        this.infoList.push({
          preset: true,
          uuid: UUID.WIFI_NAME,
          label: 'Wifi',
          value: '...'
        })
        this.infoList.push({
          preset: true,
          uuid: UUID.IP_ADDRESS,
          label: 'IP Address',
          value: '...'
        })
        await this.subscribeCharacteristic(UUID.WIFI_NAME)
        await this.subscribeCharacteristic(UUID.IP_ADDRESS)
        await this.readInfoCharacteristic(UUID.DEVICE_MODEL)
        await this.subscribeCharacteristic(UUID.NOTIFY_MESSAGE)
        await this.subscribeCharacteristic(UUID.CUSTOM_COMMAND_NOTIFY)
        this.characteristicsList.filter(i => i.uuid.indexOf(UUID.CUSTOM_INFO_LABEL) >= 0).map(item => {
          this.infoList.push({
            uuid: item.uuid.replace(UUID.CUSTOM_INFO_LABEL, UUID.CUSTOM_INFO),
            uuid_label: item.uuid,
            label: '-',
            value: ''
          })
        })
        for (let i = 0; i < this.infoList.length; i++) {
          if (this.infoList[i].preset) continue
          await this.readInfoCharacteristic(this.infoList[i].uuid_label)
          await this.subscribeCharacteristic(this.infoList[i].uuid)
        }
        this.characteristicsList.filter(i => i.uuid.indexOf(UUID.CUSTOM_COMMAND_LABEL) >= 0).map(item => {
          this.commandList.push({
            uuid: item.uuid,
            label: '...'
          })
        })
        for (let i = 0; i < this.commandList.length; i++) {
          await this.readCommandLabel(this.commandList[i].uuid)
        }
      }
    }
  }
}
</script>

<style lang="less">
  *{
    box-sizing: border-box;
  }
  html{
    width: 100%;
    height: 100%;
    background-color: orange;
    padding: 0;
    margin: 0;
  }
  body{
    padding: 10px 16px;
    margin: 0;
    background: linear-gradient(#ffe025, orange);
  }
  .logo{
    width: 160px;
  }
  #app {
    font-family: 'Avenir', Helvetica, Arial, sans-serif;
    -webkit-font-smoothing: antialiased;
    -moz-osx-font-smoothing: grayscale;
    text-align: center;
    color: #2c3e50;
    margin-top: 30px;
  }
  .panel{
    position: relative;
    width: 100%;
    min-height: 50px;
    background-color: #fff;
    margin-top: 20px;
    border-radius: 6px;
    box-shadow: 0 0 5px 2px rgba(230, 157, 42, 0.08);
    padding: 15px 20px;
    text-align: left;
    &:after{
      content: ' ';
      display: block;
      clear: both;
    }
  }
  .connect-panel{
    p{
      color: #999;
    }
  }
  .el-row{
    margin-bottom: 15px;
  }
  .command-panel{
    .button-wrap{
      float: left;
      margin: 0 10px 10px 0;
    }
  }
  .panel-title{
    width: 100%;
    height: 30px;
    text-align: left;
    font-size: 16px;
    margin-bottom: 12px;
    font-weight: bold;
  }
  .info-group{
    position: relative;
    float: left;
    display: block;
    text-align: left;
    width: 48%;
    margin-bottom: 20px;
    margin-right: 1%;
    overflow: hidden;
    min-height: 45px;
    .label{
      font-size: 14px;
      font-weight: bold;
      color: #758699;
    }
    .value{
      width: 96%;
      font-size: 14px;
      overflow:hidden;
      text-overflow:ellipsis;
    }
  }
  .code{
    display: block;
    width: 100%;
    height: 100px;
    background-color: #333333;
    color: #e4edff;
    padding: 6px 10px;
  }
  .copyright{
    width: 100%;
    color: white;
    padding: 20px 0;
    a{
      color: white;
    }
  }
  @media screen and (min-width: 900px) {
    .panel-container {
      display: flex;
      justify-content: space-between;
    }
    .panel{
      margin-right: 10px;
    }
    .copyright{
      position: fixed;
      bottom: 10px;
    }
  }
</style>
