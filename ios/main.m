#import <MetalKit/MetalKit.h>
#import <UIKit/UIKit.h>

extern void batting_ios_main(void);
extern void *batting_ios_view(void);
static BOOL needsPause = NO;
static __weak UIWindow *gameWindow;

@interface BattingMetalDelegate : NSObject <MTKViewDelegate>
@property(nonatomic, strong) id<MTKViewDelegate> renderer;
@end

@implementation BattingMetalDelegate

- (void)drawInMTKView:(MTKView *)view {
    [self.renderer drawInMTKView:view];
}

- (void)mtkView:(MTKView *)view drawableSizeWillChange:(CGSize)size {
    (void)view;
    (void)size;
    // Miniquad updates its framebuffer size at the start of each draw, but
    // 0.4.11 does not implement this required Metal delegate callback.
}

@end

@interface BattingSceneDelegate : UIResponder <UIWindowSceneDelegate>
@property(nonatomic, strong) UIWindow *window;
@property(nonatomic, strong) BattingMetalDelegate *metalDelegate;
@end

@implementation BattingSceneDelegate

- (void)scene:(UIScene *)scene willConnectToSession:(UISceneSession *)session
      options:(UISceneConnectionOptions *)connectionOptions {
    (void)session;
    (void)connectionOptions;
    if (![scene isKindOfClass:UIWindowScene.class]) {
        return;
    }

    // Miniquad 0.4.11 prepares its Metal view in the app delegate. Move that
    // controller into a scene-owned window before presenting the game.
    UIView *view = (__bridge UIView *)batting_ios_view();
    UIWindow *legacyWindow = view.window;
    UIViewController *controller = legacyWindow.rootViewController;
    legacyWindow.hidden = YES;
    legacyWindow.rootViewController = nil;

    controller.modalPresentationStyle = UIModalPresentationFullScreen;
    view.multipleTouchEnabled = NO;
    if ([view isKindOfClass:MTKView.class]) {
        MTKView *metalView = (MTKView *)view;
        self.metalDelegate = [BattingMetalDelegate new];
        self.metalDelegate.renderer = metalView.delegate;
        metalView.delegate = self.metalDelegate;
        metalView.depthStencilPixelFormat = MTLPixelFormatDepth32Float_Stencil8;
    }
    self.window = [[UIWindow alloc] initWithWindowScene:(UIWindowScene *)scene];
    self.window.rootViewController = controller;
    gameWindow = self.window;
    [self.window makeKeyAndVisible];
}

- (void)sceneDidBecomeActive:(UIScene *)scene {
    (void)scene;
    // UIKit stops sending these app callbacks after scene adoption. Forward
    // them to Miniquad so its rendering loop resumes and pauses correctly.
    UIApplication *app = UIApplication.sharedApplication;
    [app.delegate applicationDidBecomeActive:app];
}

- (void)sceneWillResignActive:(UIScene *)scene {
    (void)scene;
    needsPause = YES;
    UIApplication *app = UIApplication.sharedApplication;
    [app.delegate applicationWillResignActive:app];
}

@end

int batting_ios_metrics(float *insets) {
    UIWindow *window = gameWindow;
    UIView *view = window.rootViewController.view;
    UIEdgeInsets safe = view ? view.safeAreaInsets : UIEdgeInsetsZero;
    CGFloat scale = window.windowScene.screen.scale;
    insets[0] = safe.left * scale;
    insets[1] = safe.top * scale;
    insets[2] = safe.right * scale;
    insets[3] = safe.bottom * scale;
    BOOL active = window.windowScene.activationState == UISceneActivationStateForegroundActive;
    int flags = (active ? 1 : 0) | (needsPause ? 2 : 0);
    needsPause = NO;
    return flags;
}

int main(void) {
    @autoreleasepool {
        batting_ios_main();
    }
    return 0;
}
