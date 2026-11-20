#import <MetalKit/MetalKit.h>
#import <UIKit/UIKit.h>

extern void batting_ios_main(void);
static BOOL needsPause = NO;

int batting_ios_metrics(float *insets) {
    UIApplication *app = UIApplication.sharedApplication;
    UIWindow *window = nil;
    for (UIWindow *candidate in app.windows) {
        if (candidate.isKeyWindow) {
            window = candidate;
            break;
        }
    }
    UIView *view = window.rootViewController.view;
    view.multipleTouchEnabled = NO;
    // Miniquad creates the Metal view without an explicit depth attachment.
    if ([view isKindOfClass:MTKView.class]) {
        MTKView *metalView = (MTKView *)view;
        if (metalView.depthStencilPixelFormat != MTLPixelFormatDepth32Float_Stencil8) {
            metalView.depthStencilPixelFormat = MTLPixelFormatDepth32Float_Stencil8;
        }
    }
    UIEdgeInsets safe = view ? view.safeAreaInsets : UIEdgeInsetsZero;
    CGFloat scale = window.screen.scale;
    insets[0] = safe.left * scale;
    insets[1] = safe.top * scale;
    insets[2] = safe.right * scale;
    insets[3] = safe.bottom * scale;
    int flags = (app.applicationState == UIApplicationStateActive ? 1 : 0) | (needsPause ? 2 : 0);
    needsPause = NO;
    return flags;
}

int main(void) {
    @autoreleasepool {
        [NSNotificationCenter.defaultCenter addObserverForName:UIApplicationWillResignActiveNotification
            object:nil queue:nil usingBlock:^(NSNotification *notification) {
                (void)notification;
                needsPause = YES;
            }];
        batting_ios_main();
    }
    return 0;
}
