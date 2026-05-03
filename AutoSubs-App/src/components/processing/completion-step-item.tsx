import { Button } from "@/components/ui/button"
import {
    Item,
    ItemContent,
    ItemDescription,
    ItemFooter,
    ItemTitle,
} from "@/components/ui/item"
import { Download, List, Plus } from "lucide-react"
import { AddToTimelineDialog } from "@/components/dialogs/add-to-timeline-dialog"
import { Settings, TimelineInfo } from "@/types"
import { useResolve } from "@/contexts/ResolveContext"
import { useTranslation } from "react-i18next"

export interface CompletionStepProps {
    onExportToFile: () => void;
    onAddToTimeline: (selectedOutputTrack: string, selectedTemplate: string, presetSettings?: Record<string, unknown>) => Promise<void>;
    onViewSubtitles?: () => void;
    settings: Settings;
    timelineInfo: TimelineInfo;
    selectedIntegration?: "davinci" | "premiere" | "aftereffects";
}

export function CompletionStepItem({
    onExportToFile,
    onAddToTimeline,
    onViewSubtitles,
    settings,
    timelineInfo,
    selectedIntegration
}: CompletionStepProps) {
    const { t } = useTranslation()
    const {
        templates: resolveTemplates,
        templatesLoading: resolveTemplatesLoading,
        templatesLoaded: resolveTemplatesLoaded,
        refreshTemplates: refreshResolveTemplates,
    } = useResolve()

    const isResolveConnected = Boolean(timelineInfo?.timelineId) && selectedIntegration === "davinci"
    const isAdobeConnected = Boolean(timelineInfo?.timelineId) && (selectedIntegration === "premiere" || selectedIntegration === "aftereffects")
    const isAdobe = selectedIntegration === "premiere" || selectedIntegration === "aftereffects"


    return (
        <div className="flex w-full flex-col gap-2">
            <Item variant="default" className="bg-muted/30 border-muted-foreground/20">
                <ItemContent className="px-2">
                    <ItemTitle>
                        {t("completion.processingComplete")}
                    </ItemTitle>
                    <ItemDescription>
                        {t("completion.subtitlesReady")}
                    </ItemDescription>
                </ItemContent>
                <ItemFooter>
                    <div className="flex gap-2">
                        {onViewSubtitles && (
                            <Button
                                variant="outline"
                                size="sm"
                                onClick={onViewSubtitles}
                                className="flex items-center gap-2"
                            >
                                <List className="h-3 w-3" />
                                {t("completion.viewSubtitles")}
                            </Button>
                        )}
                        {((isResolveConnected && onViewSubtitles) || !onViewSubtitles) && (
                            <Button
                                variant="outline"
                                size="sm"
                                onClick={onExportToFile}
                                className="flex items-center gap-2"
                            >
                                <Download className="h-3 w-3" />
                                {t("completion.exportToFile")}
                            </Button>
                        )}
                        {(isResolveConnected || isAdobeConnected) && (
                            <AddToTimelineDialog
                                settings={settings}
                                timelineInfo={timelineInfo}
                                templates={isAdobe ? [] : resolveTemplates}
                                templatesLoading={!isAdobe && resolveTemplatesLoading}
                                templatesLoaded={isAdobe || resolveTemplatesLoaded}
                                onLoadTemplates={isAdobe ? undefined : refreshResolveTemplates}
                                onAddToTimeline={onAddToTimeline}
                                selectedIntegration={selectedIntegration}

                            >
                                <Button
                                    variant="outline"
                                    size="sm"
                                    className="flex items-center gap-2"
                                >
                                    <Plus className="h-3 w-3" />
                                    {t("completion.addToTimeline")}
                                </Button>
                            </AddToTimelineDialog>
                        )}
                    </div>
                </ItemFooter>
            </Item>
        </div>
    )
}
