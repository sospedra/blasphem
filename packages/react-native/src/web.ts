// Expo web and react-native-web resolve the `browser` condition here, so one
// import line serves iOS, Android, and the browser. `blasphem` is an optional
// peer: install it when the app targets the web.
export { close, createJudge, init, judge, ready } from "blasphem";
export type { Judge, JudgeOptions, Judgement } from "blasphem";
