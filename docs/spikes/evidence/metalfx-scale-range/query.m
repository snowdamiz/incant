#import <Foundation/Foundation.h>
#import <Metal/Metal.h>
#import <MetalFX/MetalFX.h>
int main(void) { @autoreleasepool {
 id<MTLDevice> device=MTLCreateSystemDefaultDevice();
 if (!device) return 2;
 if (@available(macOS 14.4, *)) {
  BOOL supported=[MTLFXTemporalScalerDescriptor supportsDevice:device];
  float min=[MTLFXTemporalScalerDescriptor supportedInputContentMinScaleForDevice:device];
  float max=[MTLFXTemporalScalerDescriptor supportedInputContentMaxScaleForDevice:device];
  NSDictionary *result=@{@"device":device.name,@"os":NSProcessInfo.processInfo.operatingSystemVersionString,@"supported":@(supported),@"raw_min_scale":@(min),@"raw_max_scale":@(max),@"gpu_submissions":@0};
  NSData *data=[NSJSONSerialization dataWithJSONObject:result options:NSJSONWritingPrettyPrinted error:nil];
  fwrite(data.bytes,1,data.length,stdout); puts(""); return 0;
 }
 return 3;
}}
